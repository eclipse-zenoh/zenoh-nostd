use zenoh_proto::fields::Encoding;

pub const ZENOH_BYTES: Encoding<'static> = Encoding::zenoh_bytes();
pub const ZENOH_STRING: Encoding<'static> = Encoding::zenoh_string();
pub const ZENOH_SERIALIZED: Encoding<'static> = Encoding::zenoh_serialized();
pub const APPLICATION_OCTET_STREAM: Encoding<'static> = Encoding::application_octet_stream();
pub const TEXT_PLAIN: Encoding<'static> = Encoding::text_plain();
pub const APPLICATION_JSON: Encoding<'static> = Encoding::application_json();
pub const TEXT_JSON: Encoding<'static> = Encoding::text_json();
pub const APPLICATION_CDR: Encoding<'static> = Encoding::application_cdr();
pub const APPLICATION_CBOR: Encoding<'static> = Encoding::application_cbor();
pub const APPLICATION_YAML: Encoding<'static> = Encoding::application_yaml();
pub const TEXT_YAML: Encoding<'static> = Encoding::text_yaml();
pub const TEXT_JSON5: Encoding<'static> = Encoding::text_json5();
pub const APPLICATION_PYTHON_SERIALIZED_OBJECT: Encoding<'static> =
    Encoding::application_python_serialized_object();
pub const APPLICATION_PROTOBUF: Encoding<'static> = Encoding::application_protobuf();
pub const APPLICATION_JAVA_SERIALIZED_OBJECT: Encoding<'static> =
    Encoding::application_java_serialized_object();
pub const APPLICATION_OPENMETRICS_TEXT: Encoding<'static> = Encoding::application_openmetrics_text();
pub const IMAGE_PNG: Encoding<'static> = Encoding::image_png();
pub const IMAGE_JPEG: Encoding<'static> = Encoding::image_jpeg();
pub const IMAGE_GIF: Encoding<'static> = Encoding::image_gif();
pub const IMAGE_BMP: Encoding<'static> = Encoding::image_bmp();
pub const IMAGE_WEBP: Encoding<'static> = Encoding::image_webp();
pub const APPLICATION_XML: Encoding<'static> = Encoding::application_xml();
pub const APPLICATION_X_WWW_FORM_URLENCODED: Encoding<'static> =
    Encoding::application_x_www_form_urlencoded();
pub const TEXT_HTML: Encoding<'static> = Encoding::text_html();
pub const TEXT_XML: Encoding<'static> = Encoding::text_xml();
pub const TEXT_CSS: Encoding<'static> = Encoding::text_css();
pub const TEXT_JAVASCRIPT: Encoding<'static> = Encoding::text_javascript();
pub const TEXT_MARKDOWN: Encoding<'static> = Encoding::text_markdown();
pub const TEXT_CSV: Encoding<'static> = Encoding::text_csv();
pub const APPLICATION_SQL: Encoding<'static> = Encoding::application_sql();
pub const APPLICATION_COAP_PAYLOAD: Encoding<'static> = Encoding::application_coap_payload();
pub const APPLICATION_JSON_PATCH_JSON: Encoding<'static> = Encoding::application_json_patch_json();
pub const APPLICATION_JSON_SEQ: Encoding<'static> = Encoding::application_json_seq();
pub const APPLICATION_JSONPATH: Encoding<'static> = Encoding::application_jsonpath();
pub const APPLICATION_JWT: Encoding<'static> = Encoding::application_jwt();
pub const APPLICATION_MP4: Encoding<'static> = Encoding::application_mp4();
pub const APPLICATION_SOAP_XML: Encoding<'static> = Encoding::application_soap_xml();
pub const APPLICATION_YANG: Encoding<'static> = Encoding::application_yang();
pub const AUDIO_AAC: Encoding<'static> = Encoding::audio_aac();
pub const AUDIO_FLAC: Encoding<'static> = Encoding::audio_flac();
pub const AUDIO_MP4: Encoding<'static> = Encoding::audio_mp4();
pub const AUDIO_OGG: Encoding<'static> = Encoding::audio_ogg();
pub const AUDIO_VORBIS: Encoding<'static> = Encoding::audio_vorbis();
pub const VIDEO_H261: Encoding<'static> = Encoding::video_h261();
pub const VIDEO_H263: Encoding<'static> = Encoding::video_h263();
pub const VIDEO_H264: Encoding<'static> = Encoding::video_h264();
pub const VIDEO_H265: Encoding<'static> = Encoding::video_h265();
pub const VIDEO_H266: Encoding<'static> = Encoding::video_h266();
pub const VIDEO_MP4: Encoding<'static> = Encoding::video_mp4();
pub const VIDEO_OGG: Encoding<'static> = Encoding::video_ogg();
pub const VIDEO_RAW: Encoding<'static> = Encoding::video_raw();
pub const VIDEO_VP8: Encoding<'static> = Encoding::video_vp8();
pub const VIDEO_VP9: Encoding<'static> = Encoding::video_vp9();

#[cfg(feature = "encoding_ext")]
pub mod ext {
    use heapless::String;
    use zenoh_proto::fields::Encoding;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EncodingError {
        UnknownEncoding,
        SchemaTooLong,
        StringConversionFailed,
    }

    pub trait EncodingExt<'a> {
        fn from_substr(s: &'a str) -> Result<Self, EncodingError>
        where
            Self: Sized;

        fn schema(&self) -> Option<&str>;

        fn set_schema(&mut self, schema: &'a str) -> Result<(), EncodingError>;

        fn to_encoding_str(&self) -> Result<String<64>, EncodingError>;
    }

    impl<'a> EncodingExt<'a> for Encoding<'a> {
        fn from_substr(s: &'a str) -> Result<Self, EncodingError> {
            if s.is_empty() {
                return Err(EncodingError::UnknownEncoding);
            }
            Ok(Encoding::parse_str(s))
        }

        fn schema(&self) -> Option<&str> {
            self.schema
                .and_then(|b| core::str::from_utf8(b).ok())
        }

        fn set_schema(&mut self, schema: &'a str) -> Result<(), EncodingError> {
            if schema.len() > 64 {
                return Err(EncodingError::SchemaTooLong);
            }
            self.schema = Some(schema.as_bytes());
            Ok(())
        }

        fn to_encoding_str(&self) -> Result<String<64>, EncodingError> {
            let s = Encoding::to_string(self);
            String::try_from(s).map_err(|_| EncodingError::StringConversionFailed)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::api::encoding::{
            APPLICATION_JSON, APPLICATION_OCTET_STREAM, TEXT_PLAIN, ZENOH_BYTES, ZENOH_STRING,
        };

        #[test]
        fn from_substr_known_encoding() {
            let enc = Encoding::from_substr("application/json").unwrap();
            assert_eq!(enc.id, APPLICATION_JSON.id);
            assert!(enc.schema.is_none());
        }

        #[test]
        fn from_substr_unknown_encoding() {
            let enc = Encoding::from_substr("my/custom").unwrap();
            assert_eq!(enc.id, 0);
            let s = enc.schema().unwrap();
            assert_eq!(s, "my/custom");
        }

        #[test]
        fn from_substr_empty_string() {
            let err = Encoding::from_substr("").unwrap_err();
            assert_eq!(err, EncodingError::UnknownEncoding);
        }

        #[test]
        fn set_schema_and_schema_accessor() {
            let mut enc = Encoding::text_plain();
            assert!(enc.schema().is_none());

            enc.set_schema("utf-8").unwrap();
            assert_eq!(enc.schema().unwrap(), "utf-8");
        }

        #[test]
        fn set_schema_roundtrip() {
            let mut enc = Encoding::application_json();
            enc.set_schema("v1").unwrap();
            let schema = enc.schema().unwrap();
            assert_eq!(schema, "v1");
        }

        #[test]
        fn to_encoding_str_known_encoding() {
            let s = APPLICATION_OCTET_STREAM.to_encoding_str().unwrap();
            assert_eq!(s, "application/octet-stream");

            let s = TEXT_PLAIN.to_encoding_str().unwrap();
            assert_eq!(s, "text/plain");

            let s = ZENOH_BYTES.to_encoding_str().unwrap();
            assert_eq!(s, "zenoh/bytes");

            let s = ZENOH_STRING.to_encoding_str().unwrap();
            assert_eq!(s, "zenoh/string");

            let s = APPLICATION_JSON.to_encoding_str().unwrap();
            assert_eq!(s, "application/json");
        }

        #[test]
        fn to_encoding_str_custom_encoding() {
            let mut enc = Encoding::text_plain();
            enc.set_schema("custom-schema").unwrap();
            let s = enc.to_encoding_str().unwrap();
            assert_eq!(s, "custom-schema");
        }

        #[test]
        fn schema_preserved_across_clone() {
            let mut enc = Encoding::application_json();
            enc.set_schema("v2").unwrap();
            let cloned = enc.clone();
            assert_eq!(cloned.schema().unwrap(), "v2");
        }

        #[test]
        fn set_schema_rejects_too_long() {
            let mut enc = Encoding::text_plain();
            let long_schema = "a".repeat(65);
            let err = enc.set_schema(&long_schema).unwrap_err();
            assert_eq!(err, EncodingError::SchemaTooLong);
        }
    }
}
