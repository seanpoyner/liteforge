use std::collections::HashMap;
use liteforge::skills::{
    extract_skill, qa_skill, rewrite_skill, summarize_skill, translate_skill,
    PromptSkill as RustPromptSkill, Skill,
};

#[napi(object)]
pub struct JsSkillConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

fn prompt_skill_to_js(s: &RustPromptSkill) -> JsSkillConfig {
    let config = s.config();
    JsSkillConfig {
        name: config.name.clone(),
        description: config.description.clone(),
        system_prompt: config.system_prompt.clone(),
        model: config.model.clone(),
        temperature: config.temperature.map(|t| t as f64),
        max_tokens: config.max_tokens,
    }
}

#[napi(object)]
pub struct JsSkillInput {
    pub text: String,
    pub parameters: HashMap<String, String>,
}

#[napi(object)]
pub struct JsSkillOutput {
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[napi]
pub fn get_summarize_skill() -> JsSkillConfig {
    prompt_skill_to_js(&summarize_skill())
}

#[napi]
pub fn get_translate_skill() -> JsSkillConfig {
    prompt_skill_to_js(&translate_skill())
}

#[napi]
pub fn get_extract_skill() -> JsSkillConfig {
    prompt_skill_to_js(&extract_skill())
}

#[napi]
pub fn get_rewrite_skill() -> JsSkillConfig {
    prompt_skill_to_js(&rewrite_skill())
}

#[napi]
pub fn get_qa_skill() -> JsSkillConfig {
    prompt_skill_to_js(&qa_skill())
}
