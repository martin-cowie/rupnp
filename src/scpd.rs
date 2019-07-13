use crate::shared::Value;
use crate::Error;
use getset::{Getters, Setters};
use serde::Deserialize;

use futures::compat::Future01CompatExt;
use futures01::{Future, Stream};

#[derive(Deserialize, Debug, Getters, Setters)]
#[serde(rename_all = "camelCase")]
pub struct SCPD {
    #[serde(skip_deserializing)]
    #[get = "pub"]
    #[set = "pub"]
    urn: String,
    service_state_table: Value<Vec<StateVariable>>,
    action_list: Value<Vec<Action>>,
}
impl SCPD {
    pub fn state_variables(&self) -> &Vec<StateVariable> {
        &self.service_state_table.value
    }
    pub fn actions(&self) -> &Vec<Action> {
        &self.action_list.value
    }

    pub fn destructure(self) -> (String, Vec<StateVariable>, Vec<Action>) {
        (
            self.urn,
            self.service_state_table.value,
            self.action_list.value,
        )
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    name: String,
    #[serde(default = "Default::default")]
    argument_list: Value<Vec<Argument>>,
}
impl Action {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn arguments(&self) -> &Vec<Argument> {
        &self.argument_list.value
    }

    pub fn input_arguments(&self) -> impl Iterator<Item = &Argument> {
        self.argument_list
            .value
            .iter()
            .filter(|arg| arg.direction.is_in())
    }
    pub fn output_arguments(&self) -> impl Iterator<Item = &Argument> {
        self.argument_list
            .value
            .iter()
            .filter(|arg| arg.direction.is_out())
    }

    pub fn destructure(self) -> (String, Vec<Argument>) {
        (self.name, self.argument_list.value)
    }
}

#[derive(Deserialize, Getters, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Argument {
    #[get = "pub"]
    name: String,
    #[get = "pub"]
    direction: Direction,
    related_state_variable: String,
}
impl Argument {
    pub fn related_state_variable(&self) -> &str {
        self.related_state_variable
            .trim_start_matches("A_ARG_TYPE_")
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    In,
    Out,
}
impl Direction {
    pub fn is_in(&self) -> bool {
        match self {
            Direction::In => true,
            Direction::Out => false,
        }
    }
    pub fn is_out(&self) -> bool {
        !self.is_in()
    }
}

#[derive(Deserialize, Getters, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StateVariable {
    name: String,
    #[serde(default = "Bool::yes")]
    ///Defines whether event messages will be generated when the value of this state variable changes.
    #[get = "pub"]
    send_events_attribute: Bool,
    #[serde(default = "Bool::no")]
    ///Defines whether event messages will be delivered using multicast eventing.
    #[get = "pub"]
    multicast: Bool,
    #[get = "pub"]
    data_type: DataType,
    #[get = "pub"]
    default_value: Option<String>,
    allowed_value_list: Option<Value<Vec<String>>>,
    #[get = "pub"]
    allowed_value_range: Option<AllowedValueRange>,
    optional: Option<()>,
}

impl StateVariable {
    pub fn name(&self) -> &str {
        self.name.trim_start_matches("A_ARG_TYPE_")
    }

    pub fn optional(&self) -> bool {
        self.optional.is_some()
    }

    pub fn allowed_values(&self) -> Option<&Vec<String>> {
        if let Some(allowed_values) = &self.allowed_value_list {
            return Some(&allowed_values.value);
        }
        None
    }

    fn data_type_str(&self) -> &str {
        if self.allowed_values().is_some() {
            self.name()
        } else {
            match self.data_type() {
                DataType::ui1 => "u8",
                DataType::ui2 => "u16",
                DataType::ui4 => "u32",
                DataType::ui8 => "u64",
                DataType::i1 => "i8",
                DataType::i2 => "i16",
                DataType::i4 => "i32",
                DataType::int => "i64",
                /* */
                DataType::char => "char",
                DataType::string => "String",
                /* */
                DataType::boolean => "upnp::datatypes::Bool",
                /* */
                DataType::uri => "hyper::Uri",
                _ => unimplemented!("{:?}", self),
            }
        }
    }

    pub fn data_type_str_input(&self) -> &str {
        self.data_type_str()
    }
    pub fn data_type_str_output(&self) -> &str {
        self.data_type_str()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Bool {
    Yes,
    No,
}
impl Bool {
    fn yes() -> Self {
        Bool::Yes
    }
    fn no() -> Self {
        Bool::No
    }
}

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types)]
pub enum DataType {
    ui1,
    ui2,
    ui4,
    ui8,
    i1,
    i2,
    i4,
    int,
    r4,
    r8,
    number,
    float,
    fixed14_4,
    char,
    string,
    date,
    dateTime,
    dateTimeTz,
    time,
    timeTz,
    boolean,
    binBase64,
    binHex,
    uri,
}

#[derive(Deserialize, Debug)]
pub struct AllowedValueRange {
    ///Inclusive lower bound
    #[serde(default = "one")]
    minimum: i32,
    ///Inclusive upper bound.
    #[serde(default = "one")]
    maximum: i32,
    #[serde(default = "one")]
    step: i32,
}
impl AllowedValueRange {
    pub fn minimum(&self) -> i32 {
        self.minimum
    }
    pub fn maximum(&self) -> i32 {
        self.maximum
    }
    pub fn step(&self) -> i32 {
        self.step
    }
}
const fn one() -> i32 {
    1
}

impl SCPD {
    pub async fn from_url(uri: hyper::Uri, urn: String) -> Result<Self, Error> {
        let client = hyper::Client::new();

        let body = client
            .get(uri)
            .and_then(|response| response.into_body().concat2())
            .map_err(Error::NetworkError)
            .compat()
            .await?;

        let mut scpd: SCPD = serde_xml_rs::from_reader(&body[..])?;
        scpd.urn = urn;
        Ok(scpd)
    }
}
