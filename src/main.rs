use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until, take_while_m_n},
    combinator::map_res,
    error::{context, convert_error},
    multi::{many_till, separated_list0, separated_list1},
    sequence::{preceded, separated_pair, terminated, tuple},
    IResult, Parser,
};
use std::error::Error;

#[derive(Debug)]
struct Name<'a> {
    family_name: &'a str,
    given_name: &'a str,
    additional_name: &'a str,
    prefix: &'a str,
    suffix: &'a str,
}

#[derive(Debug)]
struct Param<'a> {
    name: &'a str,
    value: &'a str,
}

#[derive(Debug, PartialEq)]
struct Property<'a> {
    group: Option<&'a str>,
    name: &'a str,
    params: Vec<(&'a str, &'a str)>,
    value: Vec<&'a str>,
}

#[derive(Debug)]
struct VCard<'a> {
    full_name: &'a str,
    name: Name<'a>,
    properties: Vec<Property<'a>>,
}

fn parse_vcf_begin(input: &str) -> IResult<&str, ()> {
    let (input, _) = tuple((tag_no_case("BEGIN:VCARD"), tag(LF)))(input)?;
    Ok((input, ()))
}

fn parse_vcf_end(input: &str) -> IResult<&str, ()> {
    let (input, _) = tuple((tag_no_case("END:VCARD"), tag(LF)))(input)?;
    Ok((input, ()))
}

static EQUAL: &str = "=";
static COLON: &str = ":";
static SEMI: &str = ";";
static LF: &str = "\r\n";
static COMMA: &str = ",";
static END: &str = "END";

fn parse_property_parameter(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        take_until(EQUAL),
        tag(EQUAL),
        alt((take_until(SEMI), take_until(COLON))),
    )(input)
}

fn parse_property_name(input: &str) -> IResult<&str, &str> {
    let (input, name) = alt((take_until(SEMI), take_until(COLON)))(input)?;

    if name.to_uppercase() == END {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    } else {
        Ok((input, name))
    }
}

fn parse_parameters(input: &str) -> IResult<&str, (Vec<(&str, &str)>, &str)> {
    many_till(preceded(tag(SEMI), parse_property_parameter), tag(COLON))(input)
}

fn parse_property_value(input: &str) -> IResult<&str, Vec<&str>> {
    let (input, v) = take_until(LF)(input)?;
    Ok((input, vec![v]))
}

fn parse_property(input: &str) -> IResult<&str, Property> {
    let (input, name) = parse_property_name(input)?;
    let (input, (params, _)) = parse_parameters(input)?;
    let (input, value) = parse_property_value(input)?;

    let property = Property {
        name,
        params,
        value,
        group: None,
    };

    Ok((input, property))
}

fn parse_properties(input: &str) -> IResult<&str, Vec<Property>> {
    separated_list1(tag(LF), parse_property)(input)
}

fn parse_version_3(input: &str) -> IResult<&str, usize> {
    let (input, _) = tuple((tag_no_case("VERSION:3.0"), tag(LF)))(input)?;
    Ok((input, 3))
}

fn parse(input: &str) -> IResult<&str, Vec<Property>> {
    let (input, _) = parse_vcf_begin(input)?;
    let (input, _) = parse_version_3(input)?;

    let (input, properties) = parse_properties(input)?;
    let (input, _) = tag(LF)(input)?;

    let (input, _) = parse_vcf_end(input)?;

    Ok((input, properties))
}

fn unfold(input: &mut String) {
    let mut i = 0;

    loop {
        if i >= input.len() {
            break;
        }

        if input[i..].starts_with(LF) {
            if input[i + 2..].starts_with(" ") {
                input.remove(i);
                input.remove(i);
                input.remove(i);
            }
        }

        i += 1;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut text_file = TEST_STRING.to_string();

    unfold(&mut text_file);

    let (_, properties) = parse(&text_file).map_err(|e| e.to_owned())?;

    for p in properties.iter() {
        println!("{:?}", p);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn property_parameter() {
        assert_eq!(
            parse_property_parameter("hello=test;"),
            Ok(((";"), ("hello", "test"))),
        );
        assert_eq!(
            parse_parameters(";hello=test:"),
            Ok(((";"), (vec![("hello", "test")], ""))),
        );
    }

    #[test]
    fn property_name() {
        assert_eq!(parse_property_name("check;test"), Ok(((";test"), "check")),);
        assert_eq!(parse_property_name("check:test"), Ok(((":test"), "check")),);
    }

    #[test]
    fn property_value() {
        assert_eq!(parse_property_value("test\r\n"), Ok(("\r\n", vec!["test"])),);
        assert_eq!(
            parse_property_value("hello,test\r\n"),
            Ok(("\r\n", vec!["hello,test"])),
        );
        assert_eq!(
            parse_property_value("al  hello,test\r\n"),
            Ok(("\r\n", vec!["al  hello,test"])),
        );
    }

    #[test]
    fn property() {
        let test = "FN:Hello Betty\r\nN:Hello;Betty\r\n";
        assert_eq!(
            // parse_properties("FN:Hello Betty\r\nN:Hello;Betty;;;\r\n"),
            parse_properties(test),
            Ok(("\r\n", vec![]))
        );
        assert_eq!(
            parse_property("fn:test\r\n"),
            Ok((
                "\r\n",
                Property {
                    group: None,
                    name: "fn",
                    params: vec![],
                    value: vec!["test"]
                }
            )),
        );
        assert_eq!(
            parse_property("fn;type=internet:test,time\r\n"),
            Ok((
                "\r\n",
                Property {
                    group: None,
                    name: "fn",
                    params: vec![("type", "internet")],
                    value: vec!["test,time"]
                }
            )),
        );

        assert_eq!(
            parse_properties("fn:test\r\nEND:VCARD\r\n"),
            Ok((
                "\r\nEND:VCARD\r\n",
                vec![Property {
                    group: None,
                    name: "fn",
                    params: vec![],
                    value: vec!["test"]
                }]
            )),
        );
    }
}

static TEST_STRING: &str = "BEGIN:VCARD\r
VERSION:3.0\r
FN:Hello Betty\r
N:Hello;Betty;;;\r
EMAIL;TYPE=INTERNET:hello.betty@gmail.com\r
TEL;TYPE=CELL:+91 12342 12332\r
TEL;TYPE=CELL:+1 (123) 112-123\r
ROLE:Application Engineer\r
NOTE:Gender: Male\r
PHOTO:https://lh3.googleusercontent.com/contacts/AOq4LdZ2EOkQkPc_KK2CyLAkx1\r
 8rcOgp0FYDG3f9_omOYadasd\r
CATEGORIES:myContacts\r
END:VCARD\r
";
