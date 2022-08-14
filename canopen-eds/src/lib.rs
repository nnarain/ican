// Parse a CANopen EDS file
// #![deny(missing_docs)]

use std::{
    collections::HashMap,
    fs,
    io,
    path::Path,
};

use ini::{self, Ini, Properties};
use num_traits::Num;
use lazy_static::lazy_static;
use thiserror::Error;
use regex::Regex;

// ---------------------------------------------------------------------------------------------------------------------
// Data types to represent the content of the EDS file
// ---------------------------------------------------------------------------------------------------------------------

// EDS data types
#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Bool,
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
    F32,
    VString,
    OString,
}

#[derive(Debug, Error, PartialEq)]
pub enum DataTypeError {
    #[error("Invalid data type '{0}'")]
    Invalid(u16)
}

impl TryFrom<u16> for DataType {
    type Error = DataTypeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0001 => Ok(DataType::Bool),
            0x0002 => Ok(DataType::I8),
            0x0003 => Ok(DataType::I16),
            0x0004 => Ok(DataType::I32),
            0x0005 => Ok(DataType::U8),
            0x0006 => Ok(DataType::U16),
            0x0007 => Ok(DataType::U32),
            0x0008 => Ok(DataType::F32),
            0x0009 => Ok(DataType::VString),
            0x000A => Ok(DataType::OString),
            _ => Err(DataTypeError::Invalid(value)),
        }
    }
}

/// Object access type
#[derive(Debug, PartialEq, Clone)]
pub enum AccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

/// Access type error
#[derive(Debug, Error, PartialEq)]
pub enum AccessTypeError {
    #[error("Invalid access type: '{0}'")]
    Invalid(String)
}

impl TryFrom<&str> for AccessType {
    type Error = AccessTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ro" => Ok(AccessType::ReadOnly),
            "wo" => Ok(AccessType::WriteOnly),
            "rw" => Ok(AccessType::ReadWrite),
            _ => Err(AccessTypeError::Invalid(value.to_owned())),
        }
    }
}

/// Variable type
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub parameter_name: String,
    /// Value data type
    pub data_type: DataType,
    /// Access type
    pub access_type: AccessType,
    /// Variable default value
    pub default_value: String,
    /// Whether this variable can be PDO mapped
    pub pdo_mapping: bool,
}

/// Array type
#[derive(Debug, PartialEq, Clone)]
pub struct Array {
    /// Array name
    pub parameter_name: String,
    /// Number of elements in the array
    pub subnumber: u8,
}

/// Record type
#[derive(Debug, PartialEq, Clone)]
pub struct Record {
    /// Record name
    pub parameter_name: String,
    /// Number of members in the record
    pub subnumber: u8,
}

/// EDS object types
#[derive(Debug, PartialEq)]
pub enum ObjectType {
    Variable,
    Array,
    Record,
}

/// Error when parsing Objects from EDS
#[derive(Debug, Error, PartialEq)]
pub enum ObjectTypeError {
    #[error("Invalid object type: {0}")]
    Invalid(u8)
}

impl TryFrom<u8> for ObjectType {
    type Error = ObjectTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x07 => Ok(ObjectType::Variable),
            0x08 => Ok(ObjectType::Array),
            0x09 => Ok(ObjectType::Record),
            _ => Err(ObjectTypeError::Invalid(value))
        }
    }
}

/// CANopen Object ID
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub struct CobId(u16, u8);

/// Object in the EDS
#[derive(Debug, Clone)]
pub enum Object {
    Variable(Variable),
    Array(Array),
    Record(Record),
}

#[derive(Debug, Error)]
pub enum ObjectError {
    #[error("Failed to convert")]
    FailedToConvert,
}

impl Object {
    pub fn into_variable(self) -> Result<Variable, ObjectError> {
        if let Object::Variable(var) = self {
            Ok(var)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }

    pub fn into_array(self) -> Result<Array, ObjectError> {
        if let Object::Array(arr) = self {
            Ok(arr)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }

    pub fn into_record(self) -> Result<Record, ObjectError> {
        if let Object::Record(rec) = self {
            Ok(rec)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }
}

/// EDS file representation
pub struct Eds {
    objects: HashMap<CobId, Object>,
}

/// EDS file errors
#[derive(Debug, Error)]
pub enum EdsError {
    #[error("File Error: '{0}'")]
    FileError(io::Error),
    #[error("Could not parse file: {0}")]
    ConfigError(ini::ParseError),
    #[error("Failed to convert values")]
    ConversionError,
    #[error("The section did not contain parameter {0}")]
    IncorrectProperties(String),
}

impl Eds {

    pub fn from_str(s: &str) -> Result<Eds, EdsError> {
        let ini = Ini::load_from_str(s).map_err(|e| EdsError::ConfigError(e))?;

        let mut objects: HashMap<CobId, Object> = HashMap::default();

        for (section, props) in ini.iter() {
            if let (Some(section), Some(object_type_str)) = (section, props.get("ObjectType")) {
                let value = eds_string_to_int::<u8>(object_type_str).map_err(|_| EdsError::ConversionError)?;
                let object_type = ObjectType::try_from(value).map_err(|_| EdsError::ConversionError)?;

                let object = match object_type {
                    ObjectType::Variable => Object::Variable(variable_from_props(props)?),
                    ObjectType::Array => Object::Array(array_from_props(props)?),
                    ObjectType::Record => Object::Record(record_from_props(props)?),
                };

                let cobid = eds_section_to_cobid(section)?;

                objects.insert(cobid, object);
            }
        }

        Ok(Eds{
            objects,
        })
    }

    pub fn from<P: AsRef<Path>>(file: P) -> Result<Eds, EdsError> {
        let s = fs::read_to_string(file).map_err(|e| EdsError::FileError(e))?;

        Eds::from_str(&s)
    }

    pub fn objects(&self) -> &HashMap<CobId, Object> {
        &self.objects
    }

}

fn variable_from_props(props: &Properties) -> Result<Variable, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let access_type: AccessType = props.get("AccessType")
                                       .ok_or(EdsError::IncorrectProperties(String::from("AccessType")))?
                                       .try_into()
                                       .map_err(|_| EdsError::ConversionError)?;
    let data_type: DataType = props.get("DataType")
                                   .ok_or(EdsError::IncorrectProperties(String::from("DataType")))
                                   .map(|s| eds_string_to_int::<u16>(s).map_err(|_| EdsError::ConversionError))??
                                   .try_into()
                                   .map_err(|_| EdsError::ConversionError)?;
    let default_value = props.get("DefaultValue").ok_or(EdsError::ConversionError)?.to_owned();
    let pdo_mapping = props.get("PDOMapping")
                           .ok_or(EdsError::ConversionError)
                           .map(|s| eds_string_to_int::<u8>(s)
                           .map_err(|_| EdsError::ConversionError))?? != 0;


    Ok(Variable{
        parameter_name,
        access_type,
        data_type,
        default_value,
        pdo_mapping,
    })
}

fn array_from_props(props: &Properties) -> Result<Array, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let subnumber = props.get("SubNumber")
                         .ok_or(EdsError::ConversionError)
                         .map(|s| eds_string_to_int::<u8>(s)
                         .map_err(|_| EdsError::ConversionError))??;

    Ok(Array{
        parameter_name,
        subnumber,
    })
}

fn record_from_props(props: &Properties) -> Result<Record, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let subnumber = props.get("SubNumber")
                         .ok_or(EdsError::ConversionError)
                         .map(|s| eds_string_to_int::<u8>(s)
                         .map_err(|_| EdsError::ConversionError))??;

    Ok(Record{
        parameter_name,
        subnumber,
    })
}

fn eds_section_to_cobid(section: &str) -> Result<CobId, EdsError> {
    lazy_static! {
        static ref RE_COBID: Regex = Regex::new(r"([a-zA-Z0-9]{4})(?:sub){0,1}(\d){0,1}").unwrap();
    }

    if let Some(caps) = RE_COBID.captures(section) {
        match (caps.get(1), caps.get(2)) {
            (Some(index), None) => {
                let index = u16::from_str_radix(index.as_str(), 16).map_err(|_| EdsError::ConversionError)?;
                Ok(CobId(index, 0x00))
            },
            (Some(index), Some(subindex)) => {
                let index = u16::from_str_radix(index.as_str(), 16).map_err(|_| EdsError::ConversionError)?;
                let subindex = u8::from_str_radix(subindex.as_str(), 16).map_err(|_| EdsError::ConversionError)?;

                Ok(CobId(index, subindex))
            },
            _ => {
                unreachable!()
            }
        }
    }
    else {
        Err(EdsError::ConversionError)
    }
}

fn eds_string_to_int<N: Num>(s: &str) -> Result<N, N::FromStrRadixErr> {
    let (s, radix) = if s.contains("0x") {
        (s.trim_start_matches("0x"), 16)
    }
    else {
        (s, 10)
    };

    N::from_str_radix(s, radix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_record() {
        let eds = r#"
        [1800]
        ParameterName=Foo
        ObjectType=0x9
        SubNumber=6
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let foo = eds.objects().get(&CobId(0x1800, 0x00)).unwrap();
        let array = foo.clone().into_record().unwrap();

        assert_eq!(array.parameter_name, String::from("Foo"));
        assert_eq!(array.subnumber, 0x06);
    }

    #[test]
    fn parse_array() {
        let eds = r#"
        [1600]
        ParameterName=Foo
        ObjectType=0x8
        SubNumber=5
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let foo = eds.objects().get(&CobId(0x1600, 0x00)).unwrap();
        let array = foo.clone().into_array().unwrap();

        assert_eq!(array.parameter_name, String::from("Foo"));
        assert_eq!(array.subnumber, 0x05);
    }

    #[test]
    fn parse_variable() {
        let eds = r#"
        [607C]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0004
        AccessType=rw
        DefaultValue=0
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();

        let foo = eds.objects().get(&CobId(0x607C, 0x00)).unwrap();
        let var = foo.clone().into_variable().unwrap();

        assert_eq!(var.parameter_name, String::from("Foo"));
        assert_eq!(var.data_type, DataType::I32);
        assert_eq!(var.access_type, AccessType::ReadWrite);
        assert_eq!(var.pdo_mapping, false);
    }

    #[test]
    fn section_to_cobid() {
        let section = "6000";
        let cobid = eds_section_to_cobid(section).unwrap();
        assert_eq!(cobid, CobId(0x6000, 0x00))
    }

    #[test]
    fn section_to_cobid_with_subindex() {
        let section = "6000sub2";
        let cobid = eds_section_to_cobid(section).unwrap();
        assert_eq!(cobid, CobId(0x6000, 0x02))
    }

    #[test]
    fn parse_data_type() {
        let r = DataType::try_from(0x0001);
        assert_eq!(r, Ok(DataType::Bool));

        let r = DataType::try_from(0x0002);
        assert_eq!(r, Ok(DataType::I8));

        let r = DataType::try_from(0x0003);
        assert_eq!(r, Ok(DataType::I16));

        let r = DataType::try_from(0x0004);
        assert_eq!(r, Ok(DataType::I32));

        let r = DataType::try_from(0x0005);
        assert_eq!(r, Ok(DataType::U8));

        let r = DataType::try_from(0x0006);
        assert_eq!(r, Ok(DataType::U16));

        let r = DataType::try_from(0x0007);
        assert_eq!(r, Ok(DataType::U32));

        let r = DataType::try_from(0x0008);
        assert_eq!(r, Ok(DataType::F32));

        let r = DataType::try_from(0x0009);
        assert_eq!(r, Ok(DataType::VString));

        let r = DataType::try_from(0x000A);
        assert_eq!(r, Ok(DataType::OString));
    }

    #[test]
    fn parse_access_type() {
        let r = AccessType::try_from("ro");
        assert_eq!(r, Ok(AccessType::ReadOnly));

        let r = AccessType::try_from("wo");
        assert_eq!(r, Ok(AccessType::WriteOnly));

        let r = AccessType::try_from("rw");
        assert_eq!(r, Ok(AccessType::ReadWrite));
    }

    #[test]
    fn parse_object_type() {
        let r = ObjectType::try_from(0x07);
        assert_eq!(r, Ok(ObjectType::Variable));

        let r = ObjectType::try_from(0x08);
        assert_eq!(r, Ok(ObjectType::Array));

        let r = ObjectType::try_from(0x09);
        assert_eq!(r, Ok(ObjectType::Record));
    }

    #[test]
    fn string_to_int_hex() {
        let s = "0x0A";
        let i = eds_string_to_int::<u8>(s).unwrap();
        assert_eq!(i, 10);
    }

    #[test]
    fn string_to_int_dec() {
        let s = "7";
        let i = eds_string_to_int::<u8>(s).unwrap();
        assert_eq!(i, 7);
    }

}
