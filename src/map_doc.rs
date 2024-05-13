use std::collections::HashMap;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
};

#[derive(Debug, Serialize)]
pub struct EdiMapDoc {
    pub schema_version: String,
    pub file_name: Option<String>,
    pub file_type: Option<String>,
    pub delimiters: Option<Delimiters>,
    pub map: Scope,
    pub chunks: Option<Vec<Scope>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delimiters {
    pub el: String,
    pub le: String,
    pub co: String,
    pub re: String,
    pub nl: Option<String>,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            el: String::from("*"),
            le: String::from("~"),
            co: String::from(":"),
            re: String::from(">"),
            nl: Some(String::from("\n")),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum Scope {
    Loop(Loop),
    Attach(Attach),
    Segment(Segment),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalLoop {
    pub required: bool,
    pub array: bool,
    pub name: String,
    pub context: Option<String>,
    pub components: Vec<Box<Scope>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConditionalLoop {
    pub required: bool,
    pub array: bool,
    pub name: String,
    pub cnd: String,
    pub context: Option<String>,
    pub then_components: Option<Vec<Box<Scope>>>,
    pub else_components: Option<Vec<Box<Scope>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Loop {
    Normal(NormalLoop),
    Conditional(ConditionalLoop),
}
#[derive(Debug, Serialize, Deserialize)]
pub enum AfterRule {
    SuppressSegmentIf(String),
    IfThenSuppress { cnd: String, target: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attach {
    pub required: bool,
    pub array: bool,
    pub name: String,
    pub cnd: String,
    pub attach: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalSegment {
    pub required: bool,
    pub name: String,
    pub elements: Vec<Element>,
    pub after: Option<Vec<AfterRule>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConditionalSegment {
    pub required: bool,
    pub name: String,
    pub cnd: String,
    pub then_elements: Option<Vec<Element>>,
    pub else_elements: Option<Vec<Element>>,
    pub after: Option<Vec<AfterRule>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Segment {
    Normal(NormalSegment),
    Conditional(ConditionalSegment),
}

#[derive(Debug, Serialize)]
pub struct Composite {
    pub required: bool,
    pub index: u8,
    pub mappings: Vec<BasicElement>,
}

#[derive(Debug, Serialize)]
pub enum Element {
    Normal(BasicElement),
    Composite(Composite),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicElement {
    pub required: bool,
    pub index: u8,
    pub mapping: String,
}

// ----------------- Serde -----------------
// all of this code down here is accounting for the weirdness of the
// .edi-map.yml format.

struct InnerElementSeed;
impl<'de> DeserializeSeed<'de> for InnerElementSeed {
    type Value = Element;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(InnerElementVisitor)
    }
}
struct InnerElementVisitor;
impl<'de> Visitor<'de> for InnerElementVisitor {
    type Value = Element;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "inner element")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Element::Normal(BasicElement {
            required: false,
            index: 0,
            mapping: v.to_string(),
        }))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Element::Normal(BasicElement {
            required: false,
            index: 0,
            mapping: v.to_string(),
        }))
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: de::SeqAccess<'de>,
    {
        let mut mappings: Vec<BasicElement> = Vec::new();

        while let Some(next_element) = seq.next_element::<HashMap<String, String>>()? {
            let (key, mapping) = next_element.into_iter().next().unwrap();
            let required = match key.chars().nth(1) {
                Some('R') => true,
                Some('S') => false,
                Some(b) => {
                    return Err(serde::de::Error::invalid_value(
                        de::Unexpected::Char(b),
                        &self,
                    ))
                }
                None => {
                    return Err(serde::de::Error::invalid_value(
                        de::Unexpected::Option,
                        &self,
                    ))
                }
            };
            let index: u8 = key
                .chars()
                .skip(3)
                .collect::<String>()
                .parse()
                .map_err(|_| serde::de::Error::invalid_value(Unexpected::Option, &self))?;
            mappings.push(BasicElement {
                required,
                index,
                mapping,
            });
        }

        Ok(Element::Composite(Composite {
            required: false,
            index: 0,
            mappings,
        }))
    }
}

struct ElementVisitorSeed;
impl<'de> DeserializeSeed<'de> for ElementVisitorSeed {
    type Value = Element;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ElementVisitor)
    }
}

struct ElementVisitor;
impl<'de> Visitor<'de> for ElementVisitor {
    type Value = Element;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "element")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let key = map.next_key::<String>()?.unwrap();

        let required = match key.chars().nth(1) {
            Some('R') => true,
            Some('S') => false,
            Some(b) => {
                return Err(serde::de::Error::invalid_value(
                    de::Unexpected::Char(b),
                    &self,
                ))
            }
            None => {
                return Err(serde::de::Error::invalid_value(
                    de::Unexpected::Option,
                    &self,
                ))
            }
        };

        let mut inner_element = map.next_value_seed(InnerElementSeed)?;

        match &mut inner_element {
            Element::Normal(n) => {
                n.required = required;
                n.index = key
                    .chars()
                    .skip(3)
                    .collect::<String>()
                    .parse()
                    .map_err(|_| serde::de::Error::invalid_value(Unexpected::Option, &self))?;
            }
            Element::Composite(c) => {
                c.required = required;
                c.index = key
                    .chars()
                    .skip(3)
                    .collect::<String>()
                    .parse()
                    .map_err(|_| serde::de::Error::invalid_value(Unexpected::Option, &self))?;
            }
        };

        Ok(inner_element)
    }
}

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ElementVisitor)
    }
}

struct VecBoxScopeSeed;
impl<'de> DeserializeSeed<'de> for VecBoxScopeSeed {
    type Value = Vec<Box<Scope>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecBoxScopeVisitor;
        impl<'de> Visitor<'de> for VecBoxScopeVisitor {
            type Value = Vec<Box<Scope>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "vec box scope")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: de::SeqAccess<'de>,
            {
                let mut vec: Self::Value = Vec::new();

                while let Some(next) = seq.next_element_seed(ScopeSeed)? {
                    vec.push(Box::new(next));
                }

                Ok(vec)
            }
        }

        deserializer.deserialize_seq(VecBoxScopeVisitor)
    }
}

struct InnerScopeSeed;
impl<'de> DeserializeSeed<'de> for InnerScopeSeed {
    type Value = Scope;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(InnerScopeVisitor)
    }
}

struct InnerScopeVisitor;
impl<'de> Visitor<'de> for InnerScopeVisitor {
    type Value = Scope;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "inner scope")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        enum Kind {
            Attach,
            Loop,
            Segment,
        }

        let mut kind: Option<Kind> = None;
        let mut cnd: Option<String> = None;
        let mut array: bool = false;
        let mut attach: Option<String> = None;
        let mut components: Option<Vec<Box<Scope>>> = None;
        let mut else_components: Option<Vec<Box<Scope>>> = None;
        let mut elements: Option<Vec<Element>> = None;
        let mut else_elements: Option<Vec<Element>> = None;
        let mut after: Option<Vec<AfterRule>> = None;
        let mut context: Option<String> = None;
        let name = String::new();
        let required = false;

        while let Some(k) = map.next_key::<String>()? {
            match k.as_str() {
                "if" => {
                    if cnd.is_some() {
                        return Err(de::Error::duplicate_field("if"));
                    }
                    cnd = Some(map.next_value::<String>()?);
                }
                "attach" => {
                    if attach.is_some() {
                        return Err(de::Error::duplicate_field("attach"));
                    }
                    attach = Some(map.next_value::<String>()?);
                    kind = Some(Kind::Attach);
                }
                "components" | "then_components" => {
                    if components.is_some() {
                        return Err(de::Error::duplicate_field("components"));
                    }
                    components = Some(map.next_value_seed(VecBoxScopeSeed)?);
                    kind = Some(Kind::Loop);
                }
                "else_components" => {
                    if else_components.is_some() {
                        return Err(de::Error::duplicate_field("else_components"));
                    }
                    else_components = Some(map.next_value::<Vec<Box<Scope>>>()?);
                }
                "elements" | "then_elements" => {
                    if elements.is_some() {
                        return Err(de::Error::duplicate_field("elements"));
                    }
                    elements = Some(map.next_value::<Vec<Element>>()?);
                    kind = Some(Kind::Segment);
                }
                "else_elements" => {
                    if elements.is_some() {
                        return Err(de::Error::duplicate_field("else_elements"));
                    }
                    else_elements = Some(map.next_value::<Vec<Element>>()?);
                }
                "after" => {
                    if after.is_some() {
                        return Err(de::Error::duplicate_field("after"));
                    }
                    after = Some(map.next_value::<Vec<AfterRule>>()?);
                }
                "context" => {
                    if context.is_some() {
                        return Err(de::Error::duplicate_field("context"));
                    }
                    context = Some(map.next_value::<String>()?);
                }
                "array" => {
                    array = map.next_value()?;
                }
                _ => {}
            }
        }

        Ok(match kind {
            None => return Err(de::Error::unknown_variant("None", &["Something"])),
            Some(x) if matches!(x, Kind::Attach) => Scope::Attach(Attach {
                attach: attach.unwrap(),
                cnd: cnd.unwrap(),
                name,
                required,
                array,
            }),
            Some(x) if matches!(x, Kind::Loop) && cnd.is_none() => {
                Scope::Loop(Loop::Normal(NormalLoop {
                    required,
                    array,
                    name,
                    context,
                    components: components.unwrap(),
                }))
            }
            Some(x) if matches!(x, Kind::Loop) && cnd.is_some() => {
                //todo! validations
                Scope::Loop(Loop::Conditional(ConditionalLoop {
                    cnd: cnd.unwrap(),
                    context: context,
                    else_components: else_components,
                    then_components: components,
                    name,
                    required,
                    array,
                }))
            }
            Some(x) if matches!(x, Kind::Segment) && cnd.is_none() => {
                Scope::Segment(Segment::Normal(NormalSegment {
                    required,
                    name,
                    elements: elements.unwrap(),
                    after,
                }))
            }
            Some(x) if matches!(x, Kind::Segment) && cnd.is_some() => {
                Scope::Segment(Segment::Conditional(ConditionalSegment {
                    required,
                    name,
                    cnd: cnd.unwrap(),
                    then_elements: elements,
                    else_elements,
                    after,
                }))
            }
            _ => unreachable!(),
        })
    }
}

struct ScopeSeed;
impl<'de> DeserializeSeed<'de> for ScopeSeed {
    type Value = Scope;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ScopeVisitor)
    }
}

struct ChunksSeed;
impl<'de> DeserializeSeed<'de> for ChunksSeed {
    type Value = Vec<Scope>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ChunksVisitor)
    }
}

struct ChunksVisitor;
impl<'de> Visitor<'de> for ChunksVisitor {
    type Value = Vec<Scope>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "chunks")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();

        while let Some(next) = seq.next_element_seed(ScopeSeed)? {
            vec.push(next);
        }

        Ok(vec)
    }
}

struct ScopeVisitor;
impl<'de> Visitor<'de> for ScopeVisitor {
    type Value = Scope;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "scope")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let key = map.next_key::<String>()?.unwrap();

        let required = match key.chars().nth(1) {
            Some('R') => true,
            Some('S') => false,
            Some(b) => {
                return Err(serde::de::Error::invalid_value(
                    de::Unexpected::Char(b),
                    &self,
                ))
            }
            None => {
                return Err(serde::de::Error::invalid_value(
                    de::Unexpected::Option,
                    &self,
                ))
            }
        };

        let mut inner_scope = map.next_value_seed(InnerScopeSeed)?;

        match &mut inner_scope {
            Scope::Loop(l) => match l {
                Loop::Normal(n) => {
                    n.required = required;
                    n.name = key[3..].to_string();
                }
                Loop::Conditional(c) => {
                    c.required = required;
                    c.name = key[3..].to_string();
                }
            },
            Scope::Attach(a) => {
                a.required = required;
                a.name = key[3..].to_string();
            }
            Scope::Segment(s) => match s {
                Segment::Normal(n) => {
                    n.required = required;
                    n.name = segment_name(&key[3..]);
                }
                Segment::Conditional(c) => {
                    c.required = required;
                    c.name = segment_name(&key[3..]);
                }
            },
        };

        Ok(inner_scope)
    }
}

impl<'de> Deserialize<'de> for Scope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(InnerScopeVisitor)
    }
}

// final serializer
struct EdiMapDocVisitor;

impl<'de> Visitor<'de> for EdiMapDocVisitor {
    type Value = EdiMapDoc;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "edi doc")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut schema_version: Option<String> = None;
        let mut file_name: Option<String> = None;
        let mut file_type: Option<String> = None;
        let mut delimiters: Option<Option<Delimiters>> = None;
        let mut map_bucket: Option<Scope> = None;
        let mut chunks: Option<Vec<Scope>> = None;

        while let Some(next) = map.next_key::<&str>()? {
            match next {
                "schema_version" => {
                    if schema_version.is_some() {
                        return Err(de::Error::duplicate_field("schema_version"));
                    }
                    schema_version = Some(map.next_value()?);
                }
                "file_name" => {
                    if file_name.is_some() {
                        return Err(de::Error::duplicate_field("file_name"));
                    }
                    file_name = Some(map.next_value()?);
                }
                "file_type" => {
                    if file_type.is_some() {
                        return Err(de::Error::duplicate_field("file_type"));
                    }
                    file_type = Some(map.next_value()?);
                }
                "delimiters" => {
                    if delimiters.is_some() {
                        return Err(de::Error::duplicate_field("delimiters"));
                    }
                    delimiters = Some(map.next_value()?);
                }
                "map" => {
                    if map_bucket.is_some() {
                        return Err(de::Error::duplicate_field("map"));
                    }
                    map_bucket = Some(map.next_value()?);
                }
                "chunks" => {
                    if chunks.is_some() {
                        return Err(de::Error::duplicate_field("chunks"));
                    }
                    chunks = Some(map.next_value_seed(ChunksSeed)?);
                }
                _ => {
                    map.next_value::<String>()?;
                }
            }
        }

        let schema_version =
            schema_version.ok_or_else(|| de::Error::missing_field("schema_version"))?;
        let delimiters = delimiters.unwrap_or_default();
        let map_bucket = map_bucket.ok_or_else(|| de::Error::missing_field("map_bucket"))?;

        Ok(EdiMapDoc {
            schema_version,
            file_name,
            file_type,
            delimiters,
            map: map_bucket,
            chunks,
        })
    }
}

impl<'de> Deserialize<'de> for EdiMapDoc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(EdiMapDocVisitor)
    }
}

fn segment_name(name: &str) -> String {
    let k = &name[..3];

    if k.to_string().chars().nth(2) == Some('_') {
        &k[..2]
    } else {
        &k
    }
    .to_string()
}
