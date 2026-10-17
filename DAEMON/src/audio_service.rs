use crate::audio_listener::Listener;
use std::sync::Arc;

use cpal::{Device, StreamConfig, traits::{DeviceTrait, HostTrait, StreamTrait}};
use ringbuf::{HeapRb, traits::{Consumer, Producer, Split}};

pub fn check_silence(chunk: &[i16]) -> f32 {
    let mut sum_squares = 0.0;
    for &sample in chunk {
        let s = sample as f32;
        sum_squares += s * s;
    }

    return (sum_squares / chunk.len() as f32).sqrt();
}

pub fn listen(listener: &mut Listener, chunk: &[i16]) -> Option<Vec<i16>> {

    if listener.is_recording {
        listener.process_vad_chunk(chunk)
    } else {
        listener.process_idle_chunk(chunk);
        None
    }
}

pub async fn record(microphone: &str, tx: tokio::sync::mpsc::Sender<Arc<[i16]>>) -> cpal::Stream {
    let device = find_input_device(microphone);

    println!("Using input device: {}", device);

    let rb = HeapRb::<i16>::new(16_000);
    let (producer, consumer) = rb.split();
    
    let config = build_stream_config();
    let stream = build_stream(device, config, producer).expect("Failed to build stream");

    stream.play().expect("Failed to start microphone");

    tokio::spawn(async move {
        forward_audio(consumer, tx).await;
    });

    return stream;
}

fn find_input_device(microphone: &str) -> Device {
    let host = cpal::default_host();
    let device =  host.input_devices()
        .expect("No input devices found on the system")
        .find(|d| d.to_string().contains(microphone))
        .unwrap_or_else(|| {
            eprintln!("{} not found, attempting to fallback to default...", microphone);
            host.default_input_device().expect("No default input device either")
        });

    return device;
}

fn build_stream_config() -> cpal::StreamConfig {
    cpal::StreamConfig {
        channels: 1,
        sample_rate: 48_000,
        buffer_size: cpal::BufferSize::Default,
    }
}

fn build_stream(
    device: Device, 
    config: StreamConfig,
    mut producer: impl Producer<Item = i16> + Send + 'static
) -> Result<cpal::Stream, cpal::Error> {

    return device.build_input_stream(
        config, 
        move |data: &[i16], _: &_| {
            for &sample in data.iter().step_by(3) {
                let _ = producer.try_push(sample);
            }
        }, 
        move |err| {
            let err_msg = err.to_string();
            if !err_msg.contains("underrun") && !err_msg.contains("overrun") {
                eprintln!("Error occured while streaming: {}", err_msg);
            }
        },
        None);
}

async fn forward_audio(
    mut consumer: impl Consumer<Item = i16> + Send + 'static,
    tx: tokio::sync::mpsc::Sender<Arc<[i16]>>
) {
    let mut chunk = Vec::with_capacity(512);

    loop {
        while let Some(sample) = consumer.try_pop() {
            chunk.push(sample);
            if chunk.len() == 512 {
                let _ = tx.try_send(Arc::from(chunk.as_slice()));
                chunk.clear();
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}