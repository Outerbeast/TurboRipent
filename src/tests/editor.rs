use crate::editor::controller::
{
    classnames_from_entities,
    parse_key_values,
    render_key_values
};

use crate::bsp::ent::Dictionary;

// ─── parse_key_values ────────────────────────────────────────────────────────

#[test]
fn test_parse_kv_basic()
{
    let result = parse_key_values( "key=value" );
    assert_eq!(result.len(), 1);
    assert_eq!(result.get( "key" ).unwrap(), "value");
}

#[test]
fn test_parse_kv_multiple()
{
    let result = parse_key_values( "k1=v1\nk2=v2" );
    assert_eq!(result.len(), 2);
    assert_eq!(result.get( "k1" ).unwrap(), "v1");
    assert_eq!(result.get( "k2" ).unwrap(), "v2");
}

#[test]
fn test_parse_kv_empty()
{
    let result = parse_key_values( "" );
    assert!(result.is_empty());
}

#[test]
fn test_parse_kv_no_equals()
{
    let result = parse_key_values( "noequalssign" );
    assert!(result.is_empty(), "lines without '=' should be skipped");
}

#[test]
fn test_parse_kv_whitespace()
{
    let result = parse_key_values( "  key  =  val  " );
    assert_eq!(result.len(), 1);
    assert_eq!(result.get( "key" ).unwrap(), "val");
}

#[test]
fn test_parse_kv_duplicate()
{
    let result = parse_key_values( "k=v1\nk=v2" );
    assert_eq!(result.get( "k" ).unwrap(), "v2", "last value should win");
}

#[test]
fn test_parse_kv_empty_key()
{
    let result = parse_key_values( "=value" );
    assert_eq!(result.get( "" ).unwrap(), "value", "empty key is allowed");
}

#[test]
fn test_parse_kv_empty_value()
{
    let result = parse_key_values( "key=" );
    assert_eq!(result.get( "key" ).unwrap(), "", "empty value is allowed");
}

// ─── render_key_values ───────────────────────────────────────────────────────

#[test]
fn test_render_kv_single()
{
    let mut dict = Dictionary::new();
    dict.insert( "key".into(), "value".into() );
    assert_eq!(render_key_values( &dict ), "key=value");
}

#[test]
fn test_render_kv_empty()
{
    assert_eq!(render_key_values( &Dictionary::new() ), "");
}

#[test]
fn test_render_kv_sorted()
{
    let mut dict = Dictionary::new();
    dict.insert( "z".into(), "last".into() );
    dict.insert( "a".into(), "first".into() );
    dict.insert( "m".into(), "middle".into() );
    let out = render_key_values( &dict );
    assert_eq!(out, "a=first\r\nm=middle\r\nz=last", "keys should be sorted alphabetically");
}

#[test]
fn test_render_kv_roundtrip()
{
    let mut dict = Dictionary::new();
    dict.insert( "name".into(), "test".into() );
    dict.insert( "origin".into(), "0 0 0".into() );
    dict.insert( "classname".into(), "info_player_start".into() );

    let rendered = render_key_values( &dict );
    let reparsed = parse_key_values( &rendered );
    assert_eq!(dict, reparsed, "render → parse should produce identical dict");
}

// ─── classnames_from_entities ────────────────────────────────────────────────

#[test]
fn test_classname_present()
{
    let mut dict = Dictionary::new();
    dict.insert( "classname".into(), "light".into() );
    dict.insert( "origin".into(), "0 0 0".into() );

    let names = classnames_from_entities( &[dict] );
    assert_eq!(names, vec!["light"]);
}

#[test]
fn test_classname_missing()
{
    let mut dict = Dictionary::new();
    dict.insert( "origin".into(), "0 0 0".into() );

    let names = classnames_from_entities( &[dict] );
    assert_eq!(names, vec!["<no classname>"]);
}

#[test]
fn test_classname_empty_string()
{
    let mut dict = Dictionary::new();
    dict.insert( "classname".into(), "".into() );

    let names = classnames_from_entities( &[dict] );
    assert_eq!(names, vec!["<no classname>"]);
}

#[test]
fn test_classname_empty_vec()
{
    let names = classnames_from_entities( &[] );
    assert!(names.is_empty());
}
