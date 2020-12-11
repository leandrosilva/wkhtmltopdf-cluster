use lazy_static::*;

use super::error::{AnyError, Result};
use serde_json::Value;
use std::collections::HashMap;

// libwkhtmltox
// https://wkhtmltopdf.org/libwkhtmltox/pagesettings.html

#[derive(Debug)]
pub enum PdfSettingType {
    ValueString,
    ValueBool,
    ValueInt,
    ValueUint,
    ValueFloat,
}

pub struct PdfSetting {
    pub scope: &'static str,
    pub key: &'static str,
    pub value_type: PdfSettingType,
}

lazy_static! {
    pub static ref PDF_GLOBAL_SETTINGS: HashMap<&'static str, PdfSetting> = {
        let mut m = HashMap::new();
        m.insert("size.pageSize",  PdfSetting { scope: "Global", key: "size.pageSize", value_type: PdfSettingType::ValueString });
        m.insert("size.width",     PdfSetting { scope: "Global", key: "size.width", value_type: PdfSettingType::ValueString });
        m.insert("size.height",    PdfSetting { scope: "Global", key: "size.height", value_type: PdfSettingType::ValueString });
        m.insert("orientation",    PdfSetting { scope: "Global", key: "orientation", value_type: PdfSettingType::ValueString });
        m.insert("colorMode",      PdfSetting { scope: "Global", key: "colorMode", value_type: PdfSettingType::ValueString });
        m.insert("dpi",            PdfSetting { scope: "Global", key: "dpi", value_type: PdfSettingType::ValueUint });
        m.insert("pageOffset",     PdfSetting { scope: "Global", key: "pageOffset", value_type: PdfSettingType::ValueInt });
        m.insert("copies",         PdfSetting { scope: "Global", key: "copies", value_type: PdfSettingType::ValueUint });
        m.insert("collate",        PdfSetting { scope: "Global", key: "collate", value_type: PdfSettingType::ValueBool });
        m.insert("outline",        PdfSetting { scope: "Global", key: "outline", value_type: PdfSettingType::ValueBool });
        m.insert("outlineDepth",   PdfSetting { scope: "Global", key: "outlineDepth", value_type: PdfSettingType::ValueUint });
        m.insert("dumpOutline",    PdfSetting { scope: "Global", key: "dumpOutline", value_type: PdfSettingType::ValueString });
        m.insert("out",            PdfSetting { scope: "Global", key: "out", value_type: PdfSettingType::ValueString });
        m.insert("documentTitle",  PdfSetting { scope: "Global", key: "documentTitle", value_type: PdfSettingType::ValueString });
        m.insert("useCompression", PdfSetting { scope: "Global", key: "useCompression", value_type: PdfSettingType::ValueBool });
        m.insert("margin.top",     PdfSetting { scope: "Global", key: "margin.top", value_type: PdfSettingType::ValueString });
        m.insert("margin.bottom",  PdfSetting { scope: "Global", key: "margin.bottom", value_type: PdfSettingType::ValueString });
        m.insert("margin.left",    PdfSetting { scope: "Global", key: "margin.left", value_type: PdfSettingType::ValueString });
        m.insert("margin.right",   PdfSetting { scope: "Global", key: "margin.right", value_type: PdfSettingType::ValueString });
        m.insert("imageDPI",       PdfSetting { scope: "Global", key: "imageDPI", value_type: PdfSettingType::ValueUint });
        m.insert("imageQuality",   PdfSetting { scope: "Global", key: "imageQuality", value_type: PdfSettingType::ValueUint });
        m.insert("load.cookieJar", PdfSetting { scope: "Global", key: "load.cookieJar", value_type: PdfSettingType::ValueString });
        m
    };

    pub static ref PDF_OBJECT_SETTINGS: HashMap<&'static str, PdfSetting> = {
        let mut m = HashMap::new();
        // General Settings
        m.insert("page",                           PdfSetting { scope: "Object", key: "page", value_type: PdfSettingType::ValueString });
        m.insert("useExternalLinks",               PdfSetting { scope: "Object", key: "useExternalLinks", value_type: PdfSettingType::ValueBool });
        m.insert("useLocalLinks",                  PdfSetting { scope: "Object", key: "useLocalLinks", value_type: PdfSettingType::ValueBool });
        m.insert("produceForms",                   PdfSetting { scope: "Object", key: "produceForms", value_type: PdfSettingType::ValueBool });
        m.insert("includeInOutline",               PdfSetting { scope: "Object", key: "includeInOutline", value_type: PdfSettingType::ValueBool });
        m.insert("pagesCount",                     PdfSetting { scope: "Object", key: "pagesCount", value_type: PdfSettingType::ValueBool });

        // TOC Settings
        m.insert("toc.useions.DottedLines",        PdfSetting { scope: "Object", key: "toc.useDottedLines", value_type: PdfSettingType::ValueBool });
        m.insert("toc.captionText",                PdfSetting { scope: "Object", key: "toc.captionText", value_type: PdfSettingType::ValueString });
        m.insert("toc.forwardLinks",               PdfSetting { scope: "Object", key: "toc.forwardLinks", value_type: PdfSettingType::ValueBool });
        m.insert("toc.backLinks",                  PdfSetting { scope: "Object", key: "toc.backLinks", value_type: PdfSettingType::ValueBool });
        m.insert("toc.indentation",                PdfSetting { scope: "Object", key: "toc.indentation", value_type: PdfSettingType::ValueString });
        m.insert("toc.fontScale",                  PdfSetting { scope: "Object", key: "toc.fontScale", value_type: PdfSettingType::ValueFloat });

        // Header Settings
        m.insert("header.PdfSettings.fontName",    PdfSetting { scope: "Object", key: "header.fontName", value_type: PdfSettingType::ValueString });
        m.insert("header.fontSize",                PdfSetting { scope: "Object", key: "header.fontSize", value_type: PdfSettingType::ValueString });
        m.insert("header.left",                    PdfSetting { scope: "Object", key: "header.left", value_type: PdfSettingType::ValueString });
        m.insert("header.center",                  PdfSetting { scope: "Object", key: "header.center", value_type: PdfSettingType::ValueString });
        m.insert("header.right",                   PdfSetting { scope: "Object", key: "header.right", value_type: PdfSettingType::ValueString });
        m.insert("header.line",                    PdfSetting { scope: "Object", key: "header.line", value_type: PdfSettingType::ValueBool });
        m.insert("header.spacing",                 PdfSetting { scope: "Object", key: "header.spacing", value_type: PdfSettingType::ValueFloat });
        m.insert("header.htmlUrl",                 PdfSetting { scope: "Object", key: "header.htmlUrl", value_type: PdfSettingType::ValueString });

        // Footer Settings
        m.insert("footer.ptions.fontName",         PdfSetting { scope: "Object", key: "footer.fontName", value_type: PdfSettingType::ValueString });
        m.insert("footer.fontSize",                PdfSetting { scope: "Object", key: "footer.fontSize", value_type: PdfSettingType::ValueString });
        m.insert("footer.left",                    PdfSetting { scope: "Object", key: "footer.left", value_type: PdfSettingType::ValueString });
        m.insert("footer.center",                  PdfSetting { scope: "Object", key: "footer.center", value_type: PdfSettingType::ValueString });
        m.insert("footer.right",                   PdfSetting { scope: "Object", key: "footer.right", value_type: PdfSettingType::ValueString });
        m.insert("footer.line",                    PdfSetting { scope: "Object", key: "footer.line", value_type: PdfSettingType::ValueBool });
        m.insert("footer.spacing",                 PdfSetting { scope: "Object", key: "footer.spacing", value_type: PdfSettingType::ValueFloat });
        m.insert("footer.htmlUrl",                 PdfSetting { scope: "Object", key: "footer.htmlUrl", value_type: PdfSettingType::ValueString });

        // Load Settings
        m.insert("load.usions.ername",             PdfSetting { scope: "Object", key: "load.username", value_type: PdfSettingType::ValueString });
        m.insert("load.password",                  PdfSetting { scope: "Object", key: "load.password", value_type: PdfSettingType::ValueString });
        m.insert("load.jsdelay",                   PdfSetting { scope: "Object", key: "load.jsdelay", value_type: PdfSettingType::ValueUint });
        m.insert("load.windowStatus",              PdfSetting { scope: "Object", key: "load.windowStatus", value_type: PdfSettingType::ValueString });
        m.insert("load.zoomFactor",                PdfSetting { scope: "Object", key: "load.zoomFactor", value_type: PdfSettingType::ValueString });
        m.insert("load.blockLocalFileAccess",      PdfSetting { scope: "Object", key: "load.blockLocalFileAccess", value_type: PdfSettingType::ValueString });
        m.insert("load.stopSlowScripts",           PdfSetting { scope: "Object", key: "load.stopSlowScripts", value_type: PdfSettingType::ValueBool });
        m.insert("load.loadErrorHandling",         PdfSetting { scope: "Object", key: "load.loadErrorHandling", value_type: PdfSettingType::ValueString });
        m.insert("load.proxy",                     PdfSetting { scope: "Object", key: "load.proxy", value_type: PdfSettingType::ValueString });

        // Web Settings 
        m.insert("web.bacons.kground",             PdfSetting { scope: "Object", key: "web.background", value_type: PdfSettingType::ValueBool });
        m.insert("web.loadImages",                 PdfSetting { scope: "Object", key: "web.loadImages", value_type: PdfSettingType::ValueBool });
        m.insert("web.enableJavascript",           PdfSetting { scope: "Object", key: "web.enableJavascript", value_type: PdfSettingType::ValueBool });
        m.insert("web.enableIntelligentShrinking", PdfSetting { scope: "Object", key: "web.enableIntelligentShrinking", value_type: PdfSettingType::ValueBool });
        m.insert("web.minimumFontSize",            PdfSetting { scope: "Object", key: "web.minimumFontSize", value_type: PdfSettingType::ValueUint });
        m.insert("web.defaultEncoding",            PdfSetting { scope: "Object", key: "web.defaultEncoding", value_type: PdfSettingType::ValueString });
        m.insert("web.printMediaType",             PdfSetting { scope: "Object", key: "web.printMediaType", value_type: PdfSettingType::ValueBool });
        m.insert("web.userStyleSheet",             PdfSetting { scope: "Object", key: "web.userStyleSheet", value_type: PdfSettingType::ValueString });
        m.insert("web.enablePlugins",              PdfSetting { scope: "Object", key: "web.enablePlugins", value_type: PdfSettingType::ValueBool });
        m
    };
}

pub fn get_pdf_setting_value(pdf_setting: &PdfSetting, json_value: &Value) -> Result<String> {
    let value: Option<String> = match json_value {
        Value::String(s) => {
            match pdf_setting.value_type {
                PdfSettingType::ValueString => Some(s.to_owned()),
                _ => None
            }
        },
        Value::Bool(b) => {
            match pdf_setting.value_type {
                PdfSettingType::ValueBool => Some(b.to_string()),
                _ => None
            }
        },
        Value::Number(n) => {
            match pdf_setting.value_type {
                PdfSettingType::ValueInt => Some(n.to_string()),
                PdfSettingType::ValueUint => Some(n.to_string()),
                PdfSettingType::ValueFloat => Some(n.to_string()),
                _ => None
            }
        },
        _ => None
    };

    match value {
        Some(v) => Ok(v),
        None => {
            let err_msg = build_err_msg(&pdf_setting, &json_value);
            Err(AnyError::without_parent(err_msg.as_str()))
        }
    }
}

fn build_err_msg(pdf_setting: &PdfSetting, json_value: &Value) -> String {
    format!(
        "{} setting '{}' must be of type '{:?}': {}", 
        pdf_setting.scope,
        pdf_setting.key,
        pdf_setting.value_type,
        json_value.to_string()
    )
}