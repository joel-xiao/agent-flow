use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 语音相关 API 实现

pub async fn speech_to_text(
    client: &GenericApiClient,
    request: SpeechToTextRequest,
) -> Result<SpeechToTextResponse> {
    let mut form = reqwest::multipart::Form::new()
        .part("audio", reqwest::multipart::Part::bytes(request.audio)
            .file_name("audio")
            .mime_str(request.format.as_deref().unwrap_or("audio/wav"))
            .map_err(|e| AgentFlowError::Other(anyhow!("Invalid mime type: {}", e)))?)
        .text("model", request.model);

    if let Some(language) = request.language {
        form = form.text("language", language);
    }

    let url = client.config.get_endpoint("speech_to_text")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?;

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(SpeechToTextResponse {
        text: payload["text"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing text")))?
            .to_string(),
        metadata: payload,
    })
}

pub async fn text_to_speech(
    client: &GenericApiClient,
    request: TextToSpeechRequest,
) -> Result<TextToSpeechResponse> {
    let body = json!({
        "model": request.model,
        "text": request.text,
        "voice": request.voice,
        "speed": request.speed,
        "format": request.format,
    });

    let url = client.config.get_endpoint("text_to_speech")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?;

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let audio = response.bytes().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Failed to read response: {}", e)))?
        .to_vec();
    let format = request.format.unwrap_or_else(|| "audio/mp3".to_string());

    Ok(TextToSpeechResponse {
        audio,
        format,
        metadata: json!({}),
    })
}

pub async fn clone_voice(
    client: &GenericApiClient,
    request: VoiceCloneRequest,
) -> Result<VoiceCloneResponse> {
    let mut form = reqwest::multipart::Form::new()
        .text("name", request.name);

    for (i, sample) in request.audio_samples.iter().enumerate() {
        form = form.part(
            format!("audio_{}", i),
            reqwest::multipart::Part::bytes(sample.clone())
                .file_name(format!("sample_{}.wav", i))
                .mime_str("audio/wav")
                .map_err(|e| AgentFlowError::Other(anyhow!("Invalid mime type: {}", e)))?
        );
    }

    if let Some(desc) = request.description {
        form = form.text("description", desc);
    }

    let url = client.config.get_endpoint("clone_voice")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?;

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(VoiceCloneResponse {
        voice_id: payload["voice_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing voice_id")))?
            .to_string(),
        metadata: payload,
    })
}

pub async fn list_voices(
    client: &GenericApiClient,
) -> Result<VoiceListResponse> {
    let response = client.request("GET", "list_voices", None).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let empty_vec = Vec::<serde_json::Value>::new();
    let voices = response["voices"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .map(|v| VoiceInfo {
            id: v["id"].as_str().unwrap_or("").to_string(),
            name: v["name"].as_str().unwrap_or("").to_string(),
            metadata: v.clone(),
        })
        .collect();

    Ok(VoiceListResponse { voices })
}

pub async fn delete_voice(
    client: &GenericApiClient,
    voice_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("delete_voice")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{voice_id}", voice_id);

    client.client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

