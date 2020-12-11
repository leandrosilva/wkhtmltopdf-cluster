#[macro_use]
use lazy_static::*;

use std::collections::HashMap;

// libwkhtmltox
// https://wkhtmltopdf.org/libwkhtmltox/pagesettings.html

pub enum PdfSettingType {
    ValueString,
    ValueBool,
    ValueInt,
    ValueUint,
    ValueFloat,
}

pub struct PdfSetting {
    pub name: &'static str,
    pub value_type: PdfSettingType,
}

lazy_static! {
    pub static ref PDF_GLOBAL_SETTINGS: HashMap<&'static str, PdfSetting> = {
        let mut m = HashMap::new();
        m.insert("size.pageSize",  PdfSetting { name: "size.pageSize", value_type: PdfSettingType::ValueString });
        m.insert("size.width",     PdfSetting { name: "size.width", value_type: PdfSettingType::ValueString });
        m.insert("size.height",    PdfSetting { name: "size.height", value_type: PdfSettingType::ValueString });
        m.insert("orientation",    PdfSetting { name: "orientation", value_type: PdfSettingType::ValueString });
        m.insert("colorMode",      PdfSetting { name: "colorMode", value_type: PdfSettingType::ValueString });
        m.insert("dpi",            PdfSetting { name: "dpi", value_type: PdfSettingType::ValueUint });
        m.insert("pageOffset",     PdfSetting { name: "pageOffset", value_type: PdfSettingType::ValueInt });
        m.insert("copies",         PdfSetting { name: "copies", value_type: PdfSettingType::ValueUint });
        m.insert("collate",        PdfSetting { name: "collate", value_type: PdfSettingType::ValueBool });
        m.insert("outline",        PdfSetting { name: "outline", value_type: PdfSettingType::ValueBool });
        m.insert("outlineDepth",   PdfSetting { name: "outlineDepth", value_type: PdfSettingType::ValueUint });
        m.insert("dumpOutline",    PdfSetting { name: "dumpOutline", value_type: PdfSettingType::ValueString });
        m.insert("out",            PdfSetting { name: "out", value_type: PdfSettingType::ValueString });
        m.insert("documentTitle",  PdfSetting { name: "documentTitle", value_type: PdfSettingType::ValueString });
        m.insert("useCompression", PdfSetting { name: "useCompression", value_type: PdfSettingType::ValueBool });
        m.insert("margin.top",     PdfSetting { name: "margin.top", value_type: PdfSettingType::ValueString });
        m.insert("margin.bottom",  PdfSetting { name: "margin.bottom", value_type: PdfSettingType::ValueString });
        m.insert("margin.left",    PdfSetting { name: "margin.left", value_type: PdfSettingType::ValueString });
        m.insert("margin.right",   PdfSetting { name: "margin.right", value_type: PdfSettingType::ValueString });
        m.insert("imageDPI",       PdfSetting { name: "imageDPI", value_type: PdfSettingType::ValueUint });
        m.insert("imageQuality",   PdfSetting { name: "imageQuality", value_type: PdfSettingType::ValueUint });
        m.insert("load.cookieJar", PdfSetting { name: "load.cookieJar", value_type: PdfSettingType::ValueString });
        m
    };

    pub static ref PDF_OBJECT_SETTINGS: HashMap<&'static str, PdfSetting> = {
        let mut m = HashMap::new();
        // General PdfSettings.
        m.insert("page",                           PdfSetting { name: "page", value_type: PdfSettingType::ValueString });
        m.insert("useExternalLinks",               PdfSetting { name: "useExternalLinks", value_type: PdfSettingType::ValueBool });
        m.insert("useLocalLinks",                  PdfSetting { name: "useLocalLinks", value_type: PdfSettingType::ValueBool });
        m.insert("produceForms",                   PdfSetting { name: "produceForms", value_type: PdfSettingType::ValueBool });
        m.insert("includeInOutline",               PdfSetting { name: "includeInOutline", value_type: PdfSettingType::ValueBool });
        m.insert("pagesCount",                     PdfSetting { name: "pagesCount", value_type: PdfSettingType::ValueBool });

        // TOC PdfSettings.
        m.insert("toc.useions.DottedLines",        PdfSetting { name: "toc.useDottedLines", value_type: PdfSettingType::ValueBool });
        m.insert("toc.captionText",                PdfSetting { name: "toc.captionText", value_type: PdfSettingType::ValueString });
        m.insert("toc.forwardLinks",               PdfSetting { name: "toc.forwardLinks", value_type: PdfSettingType::ValueBool });
        m.insert("toc.backLinks",                  PdfSetting { name: "toc.backLinks", value_type: PdfSettingType::ValueBool });
        m.insert("toc.indentation",                PdfSetting { name: "toc.indentation", value_type: PdfSettingType::ValueString });
        m.insert("toc.fontScale",                  PdfSetting { name: "toc.fontScale", value_type: PdfSettingType::ValueFloat });

        // Header PdfSettings.
        m.insert("header.PdfSettings.fontName",        PdfSetting { name: "header.fontName", value_type: PdfSettingType::ValueString });
        m.insert("header.fontSize",                PdfSetting { name: "header.fontSize", value_type: PdfSettingType::ValueString });
        m.insert("header.left",                    PdfSetting { name: "header.left", value_type: PdfSettingType::ValueString });
        m.insert("header.center",                  PdfSetting { name: "header.center", value_type: PdfSettingType::ValueString });
        m.insert("header.right",                   PdfSetting { name: "header.right", value_type: PdfSettingType::ValueString });
        m.insert("header.line",                    PdfSetting { name: "header.line", value_type: PdfSettingType::ValueBool });
        m.insert("header.spacing",                 PdfSetting { name: "header.spacing", value_type: PdfSettingType::ValueFloat });
        m.insert("header.htmlUrl",                 PdfSetting { name: "header.htmlUrl", value_type: PdfSettingType::ValueString });

        // Footer PdfSettings.
        m.insert("footer.ptions.fontName",         PdfSetting { name: "footer.fontName", value_type: PdfSettingType::ValueString });
        m.insert("footer.fontSize",                PdfSetting { name: "footer.fontSize", value_type: PdfSettingType::ValueString });
        m.insert("footer.left",                    PdfSetting { name: "footer.left", value_type: PdfSettingType::ValueString });
        m.insert("footer.center",                  PdfSetting { name: "footer.center", value_type: PdfSettingType::ValueString });
        m.insert("footer.right",                   PdfSetting { name: "footer.right", value_type: PdfSettingType::ValueString });
        m.insert("footer.line",                    PdfSetting { name: "footer.line", value_type: PdfSettingType::ValueBool });
        m.insert("footer.spacing",                 PdfSetting { name: "footer.spacing", value_type: PdfSettingType::ValueFloat });
        m.insert("footer.htmlUrl",                 PdfSetting { name: "footer.htmlUrl", value_type: PdfSettingType::ValueString });

        // Load PdfSettings.
        m.insert("load.usions.ername",             PdfSetting { name: "load.username", value_type: PdfSettingType::ValueString });
        m.insert("load.password",                  PdfSetting { name: "load.password", value_type: PdfSettingType::ValueString });
        m.insert("load.jsdelay",                   PdfSetting { name: "load.jsdelay", value_type: PdfSettingType::ValueUint });
        m.insert("load.windowStatus",              PdfSetting { name: "load.windowStatus", value_type: PdfSettingType::ValueString });
        m.insert("load.zoomFactor",                PdfSetting { name: "load.zoomFactor", value_type: PdfSettingType::ValueString });
        m.insert("load.blockLocalFileAccess",      PdfSetting { name: "load.blockLocalFileAccess", value_type: PdfSettingType::ValueString });
        m.insert("load.stopSlowScripts",           PdfSetting { name: "load.stopSlowScripts", value_type: PdfSettingType::ValueBool });
        m.insert("load.loadErrorHandling",         PdfSetting { name: "load.loadErrorHandling", value_type: PdfSettingType::ValueString });
        m.insert("load.proxy",                     PdfSetting { name: "load.proxy", value_type: PdfSettingType::ValueString });

        // Web PdfSettings.
        m.insert("web.bacons.kground",             PdfSetting { name: "web.background", value_type: PdfSettingType::ValueBool });
        m.insert("web.loadImages",                 PdfSetting { name: "web.loadImages", value_type: PdfSettingType::ValueBool });
        m.insert("web.enableJavascript",           PdfSetting { name: "web.enableJavascript", value_type: PdfSettingType::ValueBool });
        m.insert("web.enableIntelligentShrinking", PdfSetting { name: "web.enableIntelligentShrinking", value_type: PdfSettingType::ValueBool });
        m.insert("web.minimumFontSize",            PdfSetting { name: "web.minimumFontSize", value_type: PdfSettingType::ValueUint });
        m.insert("web.defaultEncoding",            PdfSetting { name: "web.defaultEncoding", value_type: PdfSettingType::ValueString });
        m.insert("web.printMediaType",             PdfSetting { name: "web.printMediaType", value_type: PdfSettingType::ValueBool });
        m.insert("web.userStyleSheet",             PdfSetting { name: "web.userStyleSheet", value_type: PdfSettingType::ValueString });
        m.insert("web.enablePlugins",              PdfSetting { name: "web.enablePlugins", value_type: PdfSettingType::ValueBool });
        m
    };
}
