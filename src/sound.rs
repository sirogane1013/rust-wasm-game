use anyhow::{anyhow, Result};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, AudioDestinationNode, AudioNode};

pub fn create_audio_context() -> Result<AudioContext> {
    AudioContext::new().map_err(|e| anyhow!("Could not create audio context: {:#?}", e))
}

fn create_buffer_source(ctx: &AudioContext) -> Result<AudioBufferSourceNode> {
    ctx.create_buffer_source()
        .map_err(|e| anyhow!("Error creating buffer source {:#?}", e))
}

fn connect_with_audio_node(
    buffer_source: &AudioBufferSourceNode,
    destination: &AudioDestinationNode,
) -> Result<AudioNode> {
    buffer_source
        .connect_with_audio_node(&destination)
        .map_err(|e| anyhow!("Error connecting audio source to destination {:#?}", e))
}

pub async fn decode_audio_data(
    ctx: &AudioContext,
    array_buffer: &ArrayBuffer,
) -> Result<AudioBuffer> {
    JsFuture::from(
        ctx.decode_audio_data(&array_buffer)
            .map_err(|e| anyhow!("Could not decode audio from array buffer {:#?}", e))?
    )
            .await
            .map_err(|e| anyhow!("Could not convert promise to future {:#?}", e))?
            .dyn_into()
            .map_err(|e| anyhow!("Could not cast into AudioBuffer {:#?}", e))

}

pub fn play_sound(ctx: &AudioContext, buffer: &AudioBuffer) -> Result<()> {
    let track_source = create_buffer_source(ctx)?;
    track_source.set_buffer(Some(&buffer));
    connect_with_audio_node(&track_source, &ctx.destination())?;

    track_source
        .start()
        .map_err(|e| anyhow!("Could not start sound!{:#?}", e))
}
