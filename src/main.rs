use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until, take_while_m_n},
    combinator::map_res,
    error::context,
    multi::{many_till, separated_list0, separated_list1},
    sequence::{preceded, separated_pair, terminated, Tuple},
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
    let (input, _) = tag_no_case("BEGIN:VCARD\n")(input)?;
    Ok((input, ()))
}

fn parse_vcf_end(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag_no_case("END:VCARD\n")(input)?;
    Ok((input, ()))
}

static EQUAL: &str = "=";
static COLON: &str = ":";
static SEMI: &str = ";";
static LF: &str = "\r\n";
static COMMA: &str = ",";

fn parse_property_parameter(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        take_until(EQUAL),
        tag(EQUAL),
        alt((take_until(SEMI), take_until(COLON))),
    )(input)
}

fn parse_property_name(input: &str) -> IResult<&str, &str> {
    alt((take_until(SEMI), take_until(COLON)))(input)
}

fn parse_property_value(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list0(tag(COMMA), alt((take_until(COMMA), take_until(LF))))(input)
}

fn parse_property(input: &str) -> IResult<&str, Property> {
    let (input, name) = parse_property_name(input)?;
    let (input, (params, _)) = context(
        "params",
        many_till(preceded(tag(SEMI), parse_property_parameter), tag(COLON)),
    )(input)?;
    let (input, value) = parse_property_value(input)?;

    Ok((
        input,
        Property {
            name,
            params,
            value,
            group: None,
        },
    ))
}

fn parse_version_3(input: &str) -> IResult<&str, usize> {
    let (input, _) = tag_no_case("VERSION:3.0\n")(input)?;
    Ok((input, 3))
}

fn parse(input: &str) -> IResult<&str, Vec<Property>> {
    let (input, _) = parse_vcf_begin(input)?;
    let (input, _) = parse_version_3(input)?;

    let (input, properties) = separated_list1(tag(LF), parse_property)(input)?;

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
    println!("Hello, world!");

    let mut text_file = TEST_STRING.to_string();

    unfold(&mut text_file);

    /*
    error[E0515]: cannot return value referencing local variable `text_file`
       --> src/main.rs:136:27
        |
    136 |     let (_, properties) = parse(&text_file)?;
        |                           ^^^^^^----------^^
        |                           |     |
        |                           |     `text_file` is borrowed here
        |                           returns a value referencing data owned by the current function

     */

    let (_, properties) = parse(&text_file)?;

    println!("{:?}", parse_property_name("check:test"));

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
            Ok(("\r\n", vec!["hello", "test"])),
        );
        assert_eq!(
            parse_property_value("al  hello,test\r\n"),
            Ok(("\r\n", vec!["al  hello", "test"])),
        );
    }

    #[test]
    fn property() {
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
                    value: vec!["test", "time"]
                }
            )),
        );
    }
}

static TEST_STRING: &str = "BEGIN:VCARD
VERSION:3.0
FN:Hello Betty
N:Hello;Betty;;;
EMAIL;TYPE=INTERNET:hello.betty@gmail.com
TEL;TYPE=CELL:+91 12342 12332
TEL;TYPE=CELL:+1 (123) 112-123
ROLE:Application Engineer
NOTE:Gender: Male
PHOTO:https://lh3.googleusercontent.com/contacts/AOq4LdZ2EOkQkPc_KK2CyLAkx1
 8rcOgp0FYDG3f9_omOYadasd
CATEGORIES:myContacts
END:VCARD
";
