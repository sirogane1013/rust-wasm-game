use anyhow::{anyhow, Result};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioDestinationNode, AudioNode, GainNode,
    GainOptions,
};

pub fn create_audio_context() -> Result<AudioContext> {
    AudioContext::new().map_err(|e| anyhow!("Could not create audio context: {:#?}", e))
}

fn create_buffer_source(ctx: &AudioContext) -> Result<AudioBufferSourceNode> {
    ctx.create_buffer_source()
        .map_err(|e| anyhow!("Error creating buffer source {:#?}", e))
}

fn create_gain_node(ctx: &AudioContext, gain: f32) -> Result<GainNode> {
    let gain_options = GainOptions::new();
    gain_options.set_gain(gain);
    let gain_node = GainNode::new_with_options(ctx, &gain_options)
        .map_err(|e| anyhow!("Error creating gain {:#?}", e))?;

    Ok(gain_node)
}

fn connect_audio_nodes(source: &AudioNode, destination: &AudioNode) -> Result<AudioNode> {
    source
        .connect_with_audio_node(destination)
        .map_err(|e| anyhow!("Error connecting audio nodes {:#?}", e))
}

fn create_track_source(
    ctx: &AudioContext,
    buffer: &AudioBuffer,
    gain: f32,
) -> Result<AudioBufferSourceNode> {
    let track_source = create_buffer_source(ctx)?;
    track_source.set_buffer(Some(&buffer));
    let gain = create_gain_node(ctx, gain)?;
    connect_audio_nodes(
        &connect_audio_nodes(&track_source, &gain)?,
        &ctx.destination(),
    )?;

    Ok(track_source)
}

pub async fn decode_audio_data(
    ctx: &AudioContext,
    array_buffer: &ArrayBuffer,
) -> Result<AudioBuffer> {
    JsFuture::from(
        ctx.decode_audio_data(&array_buffer)
            .map_err(|e| anyhow!("Could not decode audio from array buffer {:#?}", e))?,
    )
    .await
    .map_err(|e| anyhow!("Could not convert promise to future {:#?}", e))?
    .dyn_into()
    .map_err(|e| anyhow!("Could not cast into AudioBuffer {:#?}", e))
}

pub enum LOOPING {
    NO,
    YES,
}

pub fn play_sound(ctx: &AudioContext, buffer: &AudioBuffer, looping: LOOPING) -> Result<()> {
    let track_source = create_track_source(ctx, buffer, 0.5)?;
    if matches!(looping, LOOPING::YES) {
        track_source.set_loop(true)
    }

    track_source
        .start()
        .map_err(|e| anyhow!("Could not start sound!{:#?}", e))
}
