//! Type conversions between Rust and Java.

// Closures capturing JNI lifetimes can't be easily converted to function references.
#![allow(clippy::redundant_closure)]

use crate::error::Result;
use jni::objects::{JList, JObject, JString, JValueGen};
use jni::sys::{jint, jlong};
use jni::JNIEnv;
use liteforge::{ChatCompletion, Choice, Message, Usage};

pub fn jstring_to_string(env: &mut JNIEnv, jstr: &JString) -> Result<String> {
    let s: String = env.get_string(jstr)?.into();
    Ok(s)
}

pub fn string_to_jstring<'a>(env: &mut JNIEnv<'a>, s: &str) -> Result<JString<'a>> {
    let js = env.new_string(s)?;
    Ok(js)
}

pub fn jlist_to_messages(env: &mut JNIEnv, list: &JObject) -> Result<Vec<Message>> {
    let jlist = JList::from_env(env, list)?;
    let mut messages = Vec::new();
    let mut iter = jlist.iter(env)?;

    while let Some(obj) = iter.next(env)? {
        let message = jobject_to_message(env, &obj)?;
        messages.push(message);
    }

    Ok(messages)
}

pub fn jobject_to_message(env: &mut JNIEnv, obj: &JObject) -> Result<Message> {
    let role_obj = env.call_method(obj, "getRole", "()Ljava/lang/String;", &[])?;
    let role_jstr = JString::from(role_obj.l()?);
    let role = jstring_to_string(env, &role_jstr)?;

    let content_obj = env
        .call_method(obj, "getContent", "()Ljava/lang/String;", &[])?
        .l()?;
    let content = if content_obj.is_null() {
        None
    } else {
        let content_jstr = JString::from(content_obj);
        Some(jstring_to_string(env, &content_jstr)?)
    };

    Ok(Message {
        role,
        content,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    })
}

pub fn completion_to_jobject<'a>(
    env: &mut JNIEnv<'a>,
    completion: &ChatCompletion,
) -> Result<JObject<'a>> {
    let class = env.find_class("com/liteforge/ChatCompletion")?;

    let id = string_to_jstring(env, &completion.id)?;
    let model = string_to_jstring(env, &completion.model)?;
    let created = completion.created as jlong;

    let choices = choices_to_jlist(env, &completion.choices)?;
    let usage = completion
        .usage
        .as_ref()
        .map(|u| usage_to_jobject(env, u))
        .transpose()?
        .unwrap_or_else(|| JObject::null());

    let obj = env.new_object(
        class,
        "(Ljava/lang/String;Ljava/lang/String;JLjava/util/List;Lcom/liteforge/Usage;)V",
        &[
            JValueGen::Object(&id),
            JValueGen::Object(&model),
            JValueGen::Long(created),
            JValueGen::Object(&choices),
            JValueGen::Object(&usage),
        ],
    )?;

    Ok(obj)
}

pub fn choices_to_jlist<'a>(env: &mut JNIEnv<'a>, choices: &[Choice]) -> Result<JObject<'a>> {
    let array_list_class = env.find_class("java/util/ArrayList")?;
    let list = env.new_object(array_list_class, "()V", &[])?;

    for choice in choices {
        let choice_obj = choice_to_jobject(env, choice)?;
        env.call_method(
            &list,
            "add",
            "(Ljava/lang/Object;)Z",
            &[JValueGen::Object(&choice_obj)],
        )?;
    }

    Ok(list)
}

pub fn choice_to_jobject<'a>(env: &mut JNIEnv<'a>, choice: &Choice) -> Result<JObject<'a>> {
    let class = env.find_class("com/liteforge/Choice")?;

    let index = choice.index as jint;
    let message = message_to_jobject(env, &choice.message)?;
    let finish_reason = choice
        .finish_reason
        .as_ref()
        .map(|r| string_to_jstring(env, r))
        .transpose()?
        .map(|s| JObject::from(s))
        .unwrap_or_else(|| JObject::null());

    let obj = env.new_object(
        class,
        "(ILcom/liteforge/Message;Ljava/lang/String;)V",
        &[
            JValueGen::Int(index),
            JValueGen::Object(&message),
            JValueGen::Object(&finish_reason),
        ],
    )?;

    Ok(obj)
}

pub fn message_to_jobject<'a>(env: &mut JNIEnv<'a>, message: &Message) -> Result<JObject<'a>> {
    let class = env.find_class("com/liteforge/Message")?;

    let role = string_to_jstring(env, &message.role)?;
    let content = message
        .content
        .as_ref()
        .map(|c| string_to_jstring(env, c))
        .transpose()?
        .map(|s| JObject::from(s))
        .unwrap_or_else(|| JObject::null());

    let obj = env.new_object(
        class,
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValueGen::Object(&role), JValueGen::Object(&content)],
    )?;

    Ok(obj)
}

pub fn usage_to_jobject<'a>(env: &mut JNIEnv<'a>, usage: &Usage) -> Result<JObject<'a>> {
    let class = env.find_class("com/liteforge/Usage")?;

    let obj = env.new_object(
        class,
        "(III)V",
        &[
            JValueGen::Int(usage.prompt_tokens as jint),
            JValueGen::Int(usage.completion_tokens as jint),
            JValueGen::Int(usage.total_tokens as jint),
        ],
    )?;

    Ok(obj)
}
