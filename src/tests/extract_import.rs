use std::
{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crate::
{
    bsp::
    {
        ent::
        {
            self, is_brush_ent, parse_entity_blocks, repair, serialize_entities,
            Dictionary, ExtractTarget, ImportSource,
            EXT_BRUSH_ENT, EXT_POINT_ENT,
        },
        stats::EntityReport,
        BspFile, BspHeader, Lump, LumpIdx,
    },
    exec,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn get_test_bsp() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("_atest folder")
        .join("osprey.bsp")
}

fn tmp_path(name: &str) -> PathBuf
{
    let dir = std::env::temp_dir().join("turboripent_tests");
    let _ = fs::create_dir_all(&dir);
    dir.join(name)
}

fn strip_null(data: &[u8]) -> &str
{
    let d = if data.last() == Some(&0) { &data[..data.len() - 1] } else { data };
    std::str::from_utf8(d).unwrap()
}

fn write_raw_bsp(path: &Path, data: &[u8])
{
    fs::write(path, data).unwrap();
}

fn write_synthetic_bsp(path: &Path, ent_data: &str)
{
    let header_size: usize = 4 + 15 * 8;
    let mut bytes = vec![0u8; header_size];

    let mut w = &mut bytes[..];
    w.write_all(&30i32.to_le_bytes()).unwrap();
    let ent_bytes = if ent_data.ends_with('\0')
    {
        ent_data.as_bytes().to_vec()
    }
    else
    {
        let mut b = ent_data.as_bytes().to_vec();
        b.push(0);
        b
    };

    // Two models (each 64 bytes) so *1 is a valid reference
    let model_bytes = vec![0u8; 128];

    for i in 0..15
    {
        let (off, len) =
            if i == LumpIdx::Entities as usize
            {
                (header_size as i32, ent_bytes.len() as i32)
            }
            else if i == LumpIdx::Models as usize
            {
                ((header_size + ent_bytes.len()) as i32, model_bytes.len() as i32)
            }
            else
            {
                (0, 0)
            };
        w.write_all(&off.to_le_bytes()).unwrap();
        w.write_all(&len.to_le_bytes()).unwrap();
    }
    bytes.extend_from_slice(&ent_bytes);
    bytes.extend_from_slice(&model_bytes);
    fs::write(path, &bytes).unwrap();
}

// ─── parse_entity_blocks / serialize_entities ────────────────────────────────

#[test]
fn test_parse_single_block()
{
    let input = "{\n\"classname\" \"worldspawn\"\n\"wad\" \"test.wad\"\n}\n";
    let blocks = parse_entity_blocks(input);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "worldspawn");
    assert_eq!(blocks[0].1.get("wad").unwrap(), "test.wad");
}

#[test]
fn test_parse_multiple_blocks()
{
    let input = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"classname\" \"light\"\n\"origin\" \"128 128 0\"\n}\n";
    let blocks = parse_entity_blocks(input);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "worldspawn");
    assert_eq!(blocks[1].1.get("classname").unwrap(), "light");
}

#[test]
fn test_serialize_empty()
{
    assert_eq!(serialize_entities(&[]), "");
}

#[test]
fn test_serialize_single()
{
    let mut dict = Dictionary::new();
    dict.insert("classname".into(), "info_player_deathmatch".into());
    dict.insert("origin".into(), "0 0 0".into());
    let out = serialize_entities(&[dict]);
    assert!(out.starts_with("{\n"));
    assert!(out.contains("\"classname\" \"info_player_deathmatch\""));
    assert!(out.ends_with("}\n"));
}

#[test]
fn test_serialize_parse_roundtrip()
{
    let mut e1 = Dictionary::new();
    e1.insert("classname".into(), "worldspawn".into());
    e1.insert("wad".into(), "test.wad".into());
    let mut e2 = Dictionary::new();
    e2.insert("classname".into(), "light".into());
    e2.insert("origin".into(), "0 128 256".into());

    let serialized = serialize_entities(&[e1, e2]);
    let reparsed = parse_entity_blocks(&serialized);
    assert_eq!(reparsed.len(), 2);
    assert_eq!(reparsed[0].1.get("classname").unwrap(), "worldspawn");
    assert_eq!(reparsed[1].1.get("origin").unwrap(), "0 128 256");
}

// ─── real BSP: extract ───────────────────────────────────────────────────────

#[test]
fn test_real_extract_single() -> Result<()>
{
    let bsp = BspFile::load(&get_test_bsp())?;
    let out_path = tmp_path("real_extract.ent");
    ent::extract(&bsp, &out_path, ExtractTarget::Single)?;

    let text = fs::read_to_string(&out_path)?;
    let blocks = parse_entity_blocks(&text);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "worldspawn");
    assert!(text.contains("Osprey Attack"));
    assert!(text.contains("func_wall"));
    assert!(blocks.len() > 700, "should have many entities");

    let _ = fs::remove_file(&out_path);
    Ok(())
}

#[test]
fn test_real_extract_split() -> Result<()>
{
    let bsp = BspFile::load(&get_test_bsp())?;
    let base = tmp_path("real_extract_split");
    ent::extract(&bsp, &base, ExtractTarget::Split)?;

    let point_text = fs::read_to_string(base.with_extension(EXT_POINT_ENT))?;
    let brush_text = fs::read_to_string(base.with_extension(EXT_BRUSH_ENT))?;

    assert!(!point_text.contains("worldspawn"), "point ents should NOT contain worldspawn");
    assert!(brush_text.contains("func_wall"), "brush ents should contain func_wall");
    assert!(brush_text.contains("worldspawn"), "brush ents should contain worldspawn");
    assert!(!point_text.contains("func_wall"), "point ents should NOT contain brush entities");

    let _ = fs::remove_file(base.with_extension(EXT_POINT_ENT));
    let _ = fs::remove_file(base.with_extension(EXT_BRUSH_ENT));
    Ok(())
}

#[test]
fn test_real_extract_import_roundtrip() -> Result<()>
{
    let copy_path = tmp_path("roundtrip_copy.bsp");
    fs::copy(get_test_bsp(), &copy_path)?;

    let bsp = BspFile::load(&copy_path)?;
    let ent_path = tmp_path("roundtrip.ent");
    ent::extract(&bsp, &ent_path, ExtractTarget::Single)?;
    let original_text = fs::read_to_string(&ent_path)?;

    let bsp2 = BspFile::load(&copy_path)?;
    let bsp2 = ent::import(bsp2, ImportSource::Text(original_text.clone()))?;
    let lump_str = strip_null(bsp2.slice_lump(LumpIdx::Entities));
    let orig_blocks = parse_entity_blocks(&original_text);
    let rt_blocks = parse_entity_blocks(lump_str);

    assert_eq!(orig_blocks.len(), rt_blocks.len());
    for ((_, od), (_, rd)) in orig_blocks.iter().zip(rt_blocks.iter())
    {
        assert_eq!(od, rd, "entity dict mismatch");
    }

    let _ = fs::remove_file(&copy_path);
    let _ = fs::remove_file(&ent_path);
    Ok(())
}

// ─── import ──────────────────────────────────────────────────────────────────

#[test]
fn test_import_text_replaces_lump() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let new = "{\n\"classname\" \"light\"\n\"origin\" \"64 64 0\"\n}\n";

    let bsp_path = tmp_path("import_text.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::Text(new.into()))?;

    let lump_str = strip_null(bsp.slice_lump(LumpIdx::Entities));
    assert!(lump_str.contains("\"classname\" \"light\""));
    assert!(lump_str.contains("\"origin\" \"64 64 0\""));
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

#[test]
fn test_import_adds_null_terminator() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let new = "{\n\"classname\" \"light\"\n}";

    let bsp_path = tmp_path("import_null.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::Text(new.into()))?;

    let lump = bsp.slice_lump(LumpIdx::Entities);
    assert_eq!(lump.last(), Some(&0), "entity lump must end with null byte");
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

#[test]
fn test_import_split_combines() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let point = "{\n\"classname\" \"info_player_deathmatch\"\n\"origin\" \"0 0 0\"\n}\n";
    let brush = "{\n\"model\" \"*1\"\n\"classname\" \"func_wall\"\n}\n";

    let bsp_path = tmp_path("import_split.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let base = tmp_path("import_split");
    fs::write(base.with_extension(EXT_POINT_ENT), point)?;
    fs::write(base.with_extension(EXT_BRUSH_ENT), brush)?;

    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::Split(base))?;

    let lump_str = strip_null(bsp.slice_lump(LumpIdx::Entities));
    assert!(lump_str.contains("info_player_deathmatch"));
    assert!(lump_str.contains("func_wall"));

    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(tmp_path("import_split.entp"));
    let _ = fs::remove_file(tmp_path("import_split.entm"));
    Ok(())
}

// ─── BSP header validation ───────────────────────────────────────────────────

#[test]
fn test_load_bsp_version_29() -> Result<()>
{
    let p = tmp_path("bad_ver.bsp");
    let mut buf = vec![0u8; 124];
    (&mut buf[..]).write_all(&29i32.to_le_bytes()).unwrap();
    write_raw_bsp(&p, &buf);

    match BspFile::load(&p)
    {
        Err(e) => assert!(e.to_string().contains("Unsupported BSP version")),
        Ok(_) => panic!("expected error for version 29")
    }
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn test_load_bsp_version_zero() -> Result<()>
{
    let p = tmp_path("zero_ver.bsp");
    write_raw_bsp(&p, &[0u8; 124]);

    match BspFile::load(&p)
    {
        Err(e) => assert!(e.to_string().contains("Unsupported BSP version")),
        Ok(_) => panic!("expected error for version 0")
    }
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn test_load_bsp_truncated() -> Result<()>
{
    let p = tmp_path("truncated.bsp");
    write_raw_bsp(&p, &[0u8; 100]);

    match BspFile::load(&p)
    {
        Err(e) => assert!(e.to_string().contains("doesn't match") || e.to_string().contains("size")),
        Ok(_) => panic!("expected error for truncated file")
    }
    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn test_load_bsp_lump_out_of_bounds() -> Result<()>
{
    let p = tmp_path("lump_oob.bsp");
    let mut buf = vec![0u8; 200];
    let mut w = &mut buf[..];
    w.write_all(&30i32.to_le_bytes()).unwrap();
    for i in 0..15
    {
        let (off, len) = if i == LumpIdx::Entities as usize
        {
            (124_i32, 999_999_i32)
        }
        else
        {
            (0, 0)
        };
        w.write_all(&off.to_le_bytes()).unwrap();
        w.write_all(&len.to_le_bytes()).unwrap();
    }
    write_raw_bsp(&p, &buf);

    match BspFile::load(&p)
    {
        Err(e) => assert!(e.to_string().contains("out of bounds")),
        Ok(_) => panic!("expected error for OOB lump")
    }
    let _ = fs::remove_file(&p);
    Ok(())
}

// ─── entity text parsing edge cases ──────────────────────────────────────────

#[test]
fn test_parse_braces_in_quoted_values()
{
    let input = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"sounds\" \"{CRACK1\"\n\"classname\" \"light\"\n}\n";
    let blocks = parse_entity_blocks(input);
    assert!(blocks.iter().any(|(_, d)| d.get("classname") == Some(&"worldspawn".into())));
}

#[test]
fn test_parse_no_braces()
{
    let input = "\"classname\" \"worldspawn\"\n\"wad\" \"test.wad\"\n";
    let blocks = parse_entity_blocks(input);
    assert!(blocks.is_empty());
}

#[test]
fn test_parse_trailing_garbage()
{
    let input = "{\n\"classname\" \"worldspawn\"\n}\nsome trailing junk";
    let blocks = parse_entity_blocks(input);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "worldspawn");
}

#[test]
fn test_parse_duplicate_keys()
{
    let input = "{\n\"classname\" \"worldspawn\"\n\"classname\" \"overwritten\"\n}\n";
    let blocks = parse_entity_blocks(input);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "overwritten");
}

#[test]
fn test_parse_odd_number_of_quotes()
{
    let input = "{\n\"classname\" \"worldspawn\"\n\"orphan\n}\n";
    let blocks = parse_entity_blocks(input);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].1.get("classname").unwrap(), "worldspawn");
    assert_eq!(blocks[0].1.len(), 1, "only classname should be present");
}

// ─── error paths ─────────────────────────────────────────────────────────────

#[test]
fn test_extract_empty_lump() -> Result<()>
{
    let p = tmp_path("empty_lump.bsp");
    // BSP where Entities lump has offset=0, length=0 (genuinely empty)
    let mut buf = vec![0u8; 124];
    (&mut buf[..]).write_all(&30i32.to_le_bytes()).unwrap();
    for i in 0..15
    {
        let (off, len) = (0i32, 0i32);
        let mut w = &mut buf[4 + i * 8..];
        w.write_all(&off.to_le_bytes()).unwrap();
        w.write_all(&len.to_le_bytes()).unwrap();
    }
    write_raw_bsp(&p, &buf);

    let bsp = BspFile::load(&p)?;
    let out = tmp_path("empty_extract.ent");
    let result = ent::extract(&bsp, &out, ExtractTarget::Single);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("data size is 0"));

    let _ = fs::remove_file(&p);
    Ok(())
}

#[test]
fn test_import_split_missing_entp() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp_path = tmp_path("split_missing_entp.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let base = tmp_path("split_missing_entp");
    fs::write(base.with_extension(EXT_BRUSH_ENT), "dummy")?;

    let bsp = BspFile::load(&bsp_path)?;
    let result = ent::import(bsp, ImportSource::Split(base));
    assert!(result.is_err());
    if let Err(e) = result { assert!(e.to_string().contains("entp")); }

    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(tmp_path("split_missing_entp.entm"));
    Ok(())
}

#[test]
fn test_import_split_missing_entm() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp_path = tmp_path("split_missing_entm.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let base = tmp_path("split_missing_entm");
    fs::write(base.with_extension(EXT_POINT_ENT), "dummy")?;

    let bsp = BspFile::load(&bsp_path)?;
    let result = ent::import(bsp, ImportSource::Split(base));
    assert!(result.is_err());
    if let Err(e) = result { assert!(e.to_string().contains("entm")); }

    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(tmp_path("split_missing_entm.entp"));
    Ok(())
}

// ─── import edge cases ───────────────────────────────────────────────────────

#[test]
fn test_import_text_with_existing_null() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp_path = tmp_path("import_ex_null.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let bsp = BspFile::load(&bsp_path)?;
    let new = "{\n\"classname\" \"light\"\n}\n\0";
    let bsp = ent::import(bsp, ImportSource::Text(new.into()))?;

    let lump = bsp.slice_lump(LumpIdx::Entities);
    assert_eq!(lump.last(), Some(&0), "must end with null");
    // Count nulls: only the one we pushed, not a double
    assert_eq!(lump.iter().filter(|&&b| b == 0).count(), 1, "no double null");
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

#[test]
fn test_import_empty_text() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp_path = tmp_path("import_empty.bsp");
    write_synthetic_bsp(&bsp_path, old);

    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::Text(String::new()))?;

    let lump = bsp.slice_lump(LumpIdx::Entities);
    // Empty text becomes just the null terminator
    assert_eq!(lump, &[0u8], "empty text should produce single null byte");
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

// ─── split extremes ──────────────────────────────────────────────────────────

#[test]
fn test_extract_split_all_point() -> Result<()>
{
    let entities = "{\n\"classname\" \"worldspawn\"\n\"wad\" \"test.wad\"\n}\n{\n\"classname\" \"light\"\n\"origin\" \"0 0 0\"\n}\n";
    let bsp_path = tmp_path("all_point.bsp");
    write_synthetic_bsp(&bsp_path, entities);

    let bsp = BspFile::load(&bsp_path)?;
    let base = tmp_path("all_point_result");
    ent::extract(&bsp, &base, ExtractTarget::Split)?;

    let point_text = fs::read_to_string(base.with_extension(EXT_POINT_ENT))?;
    let brush_text = fs::read_to_string(base.with_extension(EXT_BRUSH_ENT))?;

    assert!(!point_text.contains("worldspawn"));
    assert!(point_text.contains("light"));
    assert!(brush_text.contains("worldspawn"));
    assert!(!brush_text.contains("light"));

    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(base.with_extension(EXT_POINT_ENT));
    let _ = fs::remove_file(base.with_extension(EXT_BRUSH_ENT));
    Ok(())
}

// ─── collect_bsps ────────────────────────────────────────────────────────────

fn write_bsp_file(path: &Path)
{
    write_synthetic_bsp( path, "{\n\"classname\" \"worldspawn\"\n}\n" );
}

fn temp_dir(name: &str) -> PathBuf
{
    let dir = std::env::temp_dir().join( "turboripent_test" ).join( name );
    let _ = fs::remove_dir_all( &dir );
    fs::create_dir_all( &dir ).unwrap();
    dir
}

#[test]
fn test_collect_bsps_directory()
{
    let dir = temp_dir( "collect_bsps_dir" );
    for i in 1..=3
    {
        write_bsp_file( &dir.join( format!( "map{i}.bsp" ) ) );
    }
    // Non-.bsp files should be ignored
    fs::write( dir.join( "notes.txt" ), "not a bsp" ).unwrap();
    fs::write( dir.join( "map.map" ), "not a bsp" ).unwrap();

    let bsps = exec::collect_bsps( &dir );
    let mut names: Vec<String> = bsps.iter()
        .filter_map( |p| p.file_stem().and_then( |s| s.to_str().map( String::from ) ) )
        .collect();
    names.sort();
    assert_eq!( names, vec!["map1", "map2", "map3"] );
}

#[test]
fn test_collect_bsps_single_file()
{
    let dir = temp_dir( "collect_bsps_single" );
    write_bsp_file( &dir.join( "level.bsp" ) );

    let bsps = exec::collect_bsps( dir.join( "level.bsp" ) );
    assert_eq!( bsps.len(), 1 );
    assert_eq!( bsps[0].file_name().unwrap(), "level.bsp" );
}

#[test]
fn test_collect_bsps_empty_directory()
{
    let dir = temp_dir( "collect_bsps_empty" );
    let bsps = exec::collect_bsps( &dir );
    assert!( bsps.is_empty() );
}

// ─── rip ─────────────────────────────────────────────────────────────────────

#[test]
fn test_rip_bsp_extract() -> Result<()>
{
    let dir = temp_dir( "rip_bsp_extract" );
    let bsp_path = dir.join( "level.bsp" );
    write_synthetic_bsp( &bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n" );

    crate::bsp::ent::rip( &bsp_path )?;

    let ent_path = dir.join( "level.ent" );
    assert!( ent_path.exists(), ".ent file should be created" );

    let text = fs::read_to_string( &ent_path )?;
    assert!( text.contains( "worldspawn" ) );

    let _ = fs::remove_file( &bsp_path );
    let _ = fs::remove_file( &ent_path );
    Ok(())
}

#[test]
fn test_rip_ent_import() -> Result<()>
{
    let dir = temp_dir( "rip_ent_import" );
    let bsp_path = dir.join( "office.bsp" );
    let ent_path = dir.join( "office.ent" );

    write_synthetic_bsp( &bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n" );
    fs::write( &ent_path, "{\n\"classname\" \"info_player_start\"\n}\n" )?;

    crate::bsp::ent::rip( &ent_path )?;

    assert!( !ent_path.exists(), ".ent should be removed after import" );

    let reloaded = BspFile::load( &bsp_path )?;
    let ent_text = strip_null( reloaded.slice_lump( LumpIdx::Entities ) );
    assert!( ent_text.contains( "info_player_start" ), "bsp should contain imported entity text" );

    let _ = fs::remove_file( &bsp_path );
    Ok(())
}

#[test]
fn test_rip_entp_import() -> Result<()>
{
    let dir = temp_dir( "rip_entp_import" );
    let base = dir.join( "de_dust" );
    let bsp_path = base.with_extension( "bsp" );
    let entp_path = base.with_extension( EXT_POINT_ENT );
    let entm_path = base.with_extension( EXT_BRUSH_ENT );

    write_synthetic_bsp( &bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n" );
    fs::write( &entp_path, "{\n\"classname\" \"light\"\n}\n" )?;
    fs::write( &entm_path, "{\n\"classname\" \"worldspawn\"\n}\n" )?;

    crate::bsp::ent::rip( &entp_path )?;

    assert!( !entp_path.exists(), ".entp should be removed" );
    assert!( !entm_path.exists(), ".entm should be removed" );

    let reloaded = BspFile::load( &bsp_path )?;
    let ent_text = strip_null( reloaded.slice_lump( LumpIdx::Entities ) );
    assert!( ent_text.contains( "light" ), "bsp should contain point entity" );
    assert!( ent_text.contains( "worldspawn" ), "bsp should contain brush entity" );

    let _ = fs::remove_file( &bsp_path );
    Ok(())
}

#[test]
fn test_rip_non_existent()
{
    let dir = temp_dir( "rip_non_existent" );
    let missing = dir.join( "does_not_exist.bsp" );
    let result = crate::bsp::ent::rip( &missing );
    assert!( result.is_err(), "rip on non-existent file should return error" );
}

// ─── validate_and_fix_entity_text ─────────────────────────────────────────────

#[test]
fn test_validate_missing_trailing_quote()
{
    let input = "{\n\"origin\" \"128 64 256\n}\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks[0].1.get( "origin" ).unwrap(), "128 64 256" );
}

#[test]
fn test_validate_inner_quotes()
{
    let input = "{\n\"message\" \"He said \"hello\"\"\n}\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert!( !blocks[0].1.get( "message" ).unwrap().contains( '"' ) );
}

#[test]
fn test_validate_outside_braces_stripped()
{
    let input = "junk\n{\n\"classname\" \"worldspawn\"\n}\ntrash\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks.len(), 1 );
}

#[test]
fn test_import_discards_bad_model_ref() -> Result<()>
{
    let bsp_path = tmp_path( "bad_model.bsp" );
    write_synthetic_bsp( &bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n" );
    let bad = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"model\" \"*999\"\n\"classname\" \"bad\"\n}\n";
    let bsp = BspFile::load( &bsp_path )?;
    let bsp = ent::import( bsp, ImportSource::Text( bad.into() ) )?;
    let text = strip_null( bsp.slice_lump( LumpIdx::Entities ) );
    assert!( !text.contains( "*999" ) );
    let _ = fs::remove_file( &bsp_path );
    Ok(())
}

#[test]
fn test_validate_tight_packing()
{
    let input = "{\n\"classname\" \"worldspawn\"\n}{\n\"classname\" \"light\"\n}\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks.len(), 2 );
}

#[test]
fn test_validate_single_line_block()
{
    let input = "{\"classname\" \"light\" \"origin\" \"0 0 0\"}\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks.len(), 1 );
    assert_eq!( blocks[0].1.get( "classname" ).unwrap(), "light" );
}

#[test]
fn test_validate_dangling_brace()
{
    let input = "{\n\"classname\" \"worldspawn\"\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks.len(), 1 );
    assert_eq!( blocks[0].1.get( "classname" ).unwrap(), "worldspawn" );
}

#[test]
fn test_validate_crlf()
{
    let input = "{\r\n\"classname\" \"worldspawn\"\r\n}\r\n";
    let fixed = crate::bsp::ent::normalise_entities( input );
    let blocks = crate::bsp::ent::parse_entity_blocks( &fixed );
    assert_eq!( blocks.len(), 1 );
    assert_eq!( blocks[0].1.get( "classname" ).unwrap(), "worldspawn" );
}

// ─── is_brush_ent ────────────────────────────────────────────────────────────

#[test]
fn test_is_brush_ent_worldspawn()
{
    let mut dict = Dictionary::new();
    dict.insert( "classname".into(), "worldspawn".into() );
    assert!( is_brush_ent( &dict ) );
}

#[test]
fn test_is_brush_ent_model_star_valid()
{
    let mut dict = Dictionary::new();
    dict.insert( "classname".into(), "func_wall".into() );
    dict.insert( "model".into(), "*1".into() );
    assert!( is_brush_ent( &dict ) );
}

#[test]
fn test_is_brush_ent_point_entity()
{
    let mut dict = Dictionary::new();
    dict.insert( "classname".into(), "light".into() );
    dict.insert( "origin".into(), "0 0 0".into() );
    assert!( !is_brush_ent( &dict ) );
}

#[test]
fn test_is_brush_ent_empty_dict()
{
    assert!( !is_brush_ent( &Dictionary::new() ) );
}

#[test]
fn test_is_brush_ent_model_star_alone()
{
    let mut dict = Dictionary::new();
    dict.insert( "model".into(), "*".into() );
    assert!( !is_brush_ent( &dict ) );
}

#[test]
fn test_is_brush_ent_no_relevant_keys()
{
    let mut dict = Dictionary::new();
    dict.insert( "origin".into(), "0 0 0".into() );
    assert!( !is_brush_ent( &dict ) );
}

// ─── Lump::range ─────────────────────────────────────────────────────────────

#[test]
fn test_lump_range_normal()
{
    let l = Lump( 100, 50 );
    assert_eq!( l.range(), 100..150 );
}

#[test]
fn test_lump_range_zero_length()
{
    let l = Lump( 100, 0 );
    assert_eq!( l.range(), 100..100 );
}

#[test]
fn test_lump_length()
{
    let l = Lump( 10, 42 );
    assert_eq!( l.length(), 42 );
}

// ─── BspHeader::from_bytes ───────────────────────────────────────────────────

#[test]
fn test_header_from_bytes_valid()
{
    let mut buf = vec![0u8; 4 + 15 * 8];
    buf[..4].copy_from_slice( &30i32.to_le_bytes() );
    let header = BspHeader::from_bytes( &buf ).unwrap();
    assert_eq!( header.version, 30 );
    assert_eq!( header.lumps[0].0, 0 );
}

#[test]
fn test_header_from_bytes_empty_input()
{
    let result = BspHeader::from_bytes( &[] );
    assert!( result.is_err() );
    assert!( result.unwrap_err().to_string().contains( "File size doesn't match" ) );
}

#[test]
fn test_header_from_bytes_wrong_version()
{
    let buf = vec![0u8; 4 + 15 * 8];
    let result = BspHeader::from_bytes( &buf );
    assert!( result.is_err() );
    assert!( result.unwrap_err().to_string().contains( "Unsupported BSP version" ) );
}

#[test]
fn test_header_from_bytes_negative_lump_values()
{
    let mut buf = vec![0u8; 4 + 15 * 8];
    buf[..4].copy_from_slice( &30i32.to_le_bytes() );
    let mut w = &mut buf[4..];
    w.write_all( &(-1i32).to_le_bytes() ).unwrap();
    w.write_all( &100i32.to_le_bytes() ).unwrap();
    let header = BspHeader::from_bytes( &buf ).unwrap();
    assert_eq!( header.lumps[0].0, -1 );
    assert_eq!( header.lumps[0].1, 100 );
}

// ─── EntityReport ────────────────────────────────────────────────────────────

fn make_bsp_for_report(ent_text: &str, model_count: usize) -> crate::bsp::BspFile
{
    let header_size = 4 + 15 * 8;
    let mut ent_bytes = ent_text.as_bytes().to_vec();
    if !ent_bytes.is_empty() && ent_bytes.last() != Some(&0)
    {
        ent_bytes.push(0);
    }
    let model_bytes = vec![0u8; model_count * 64];
    let content_len = header_size + ent_bytes.len() + model_bytes.len();
    let mut content = vec![0u8; content_len];

    content[..4].copy_from_slice(&30i32.to_le_bytes());

    let mut lumps = [Lump(0, 0); 15];
    lumps[LumpIdx::Entities as usize] = Lump(header_size as i32, ent_bytes.len() as i32);
    lumps[LumpIdx::Models as usize] = Lump((header_size + ent_bytes.len()) as i32, model_bytes.len() as i32);

    for i in 0..15
    {
        let offset = 4 + i * 8;
        content[offset..offset + 4].copy_from_slice(&lumps[i].0.to_le_bytes());
        content[offset + 4..offset + 8].copy_from_slice(&lumps[i].1.to_le_bytes());
    }

    content[header_size..header_size + ent_bytes.len()].copy_from_slice(&ent_bytes);

    let header = BspHeader { version: 30, lumps };
    crate::bsp::BspFile { header, content, path: PathBuf::from("test.bsp") }
}

#[test]
fn test_entity_report_generate_basic()
{
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"classname\" \"light\"\n\"origin\" \"0 0 0\"\n}\n";
    let bsp = make_bsp_for_report(entities, 1);
    let report = EntityReport::generate(&bsp);
    assert_eq!(report.total_entities, 2);
    assert_eq!(report.point_entities, 1);
    assert_eq!(report.brush_entities, 1);
    assert_eq!(report.total_brush_models, 1);
    assert!(report.unused_model_indices.is_empty());
}

#[test]
fn test_entity_report_unused_model_indices()
{
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"classname\" \"func_wall\"\n\"model\" \"*1\"\n}\n";
    let bsp = make_bsp_for_report(entities, 3);
    let report = EntityReport::generate(&bsp);
    assert_eq!(report.total_entities, 2);
    assert_eq!(report.total_brush_models, 3);
    assert_eq!(report.unused_model_indices, vec![2]);
}

#[test]
fn test_entity_report_no_unused_models()
{
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"classname\" \"func_wall\"\n\"model\" \"*1\"\n}\n";
    let bsp = make_bsp_for_report(entities, 2);
    let report = EntityReport::generate(&bsp);
    assert_eq!(report.total_entities, 2);
    assert_eq!(report.total_brush_models, 2);
    assert!(report.unused_model_indices.is_empty());
}

#[test]
fn test_entity_report_display_none()
{
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp = make_bsp_for_report(entities, 1);
    let report = EntityReport::generate(&bsp);
    let out = report.to_string();
    assert!(out.contains("(none)"));
}

#[test]
fn test_entity_report_display_some()
{
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n";
    let bsp = make_bsp_for_report(entities, 3);
    let report = EntityReport::generate(&bsp);
    let out = report.to_string();
    assert!(out.contains("model *1 exists"));
    assert!(out.contains("model *2 exists"));
}

// ─── repair ──────────────────────────────────────────────────────────────────

#[test]
fn test_repair_ent_file() -> Result<()>
{
    let path = tmp_path("repair_test.ent");
    let malformed = "{\"classname\" \"light\" \"origin\" \"0 0 0\"}\n";
    fs::write(&path, malformed)?;
    repair(&path)?;
    let fixed = fs::read_to_string(&path)?;
    assert!(fixed.contains("{\n"));
    assert!(fixed.contains("\"classname\" \"light\""));
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn test_repair_unsupported_extension() -> Result<()>
{
    let path = tmp_path("repair_test.txt");
    fs::write(&path, "dummy")?;
    let result = repair(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported file type"));
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn test_repair_no_extension() -> Result<()>
{
    let path = tmp_path("repair_no_ext");
    fs::write(&path, "dummy")?;
    let result = repair(&path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported file type"));
    let _ = fs::remove_file(&path);
    Ok(())
}

// ─── ImportSource::File ──────────────────────────────────────────────────────

#[test]
fn test_import_source_file() -> Result<()>
{
    let old = "{\n\"classname\" \"worldspawn\"\n}\n";
    let new = "{\n\"classname\" \"light\"\n\"origin\" \"64 64 0\"\n}\n";
    let bsp_path = tmp_path("import_file.bsp");
    write_synthetic_bsp(&bsp_path, old);
    let ent_path = tmp_path("import_file.ent");
    fs::write(&ent_path, new)?;
    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::File(ent_path))?;
    let lump_str = strip_null(bsp.slice_lump(LumpIdx::Entities));
    assert!(lump_str.contains("\"classname\" \"light\""));
    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(tmp_path("import_file.ent"));
    Ok(())
}

#[test]
fn test_import_source_file_not_found() -> Result<()>
{
    let bsp_path = tmp_path("import_file_not_found.bsp");
    write_synthetic_bsp(&bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n");
    let bsp = BspFile::load(&bsp_path)?;
    let missing = tmp_path("nonexistent.ent");
    let result = ent::import(bsp, ImportSource::File(missing));
    assert!(result.is_err());
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

// ─── import edge cases ───────────────────────────────────────────────────────

#[test]
fn test_import_all_entities_filtered() -> Result<()>
{
    let bsp_path = tmp_path("all_filtered.bsp");
    write_synthetic_bsp(&bsp_path, "{\n\"classname\" \"worldspawn\"\n}\n");
    let bad = "{\n\"classname\" \"bad\"\n\"model\" \"*999\"\n}\n";
    let bsp = BspFile::load(&bsp_path)?;
    let bsp = ent::import(bsp, ImportSource::Text(bad.into()))?;
    let lump = bsp.slice_lump(LumpIdx::Entities);
    assert_eq!(lump, &[0u8], "all entities filtered should produce single null byte");
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

// ─── extract edge cases ──────────────────────────────────────────────────────

#[test]
fn test_extract_entity_lump_non_utf8() -> Result<()>
{
    let header_size = 4 + 15 * 8;
    let mut bytes = vec![0u8; header_size + 10];
    let mut w = &mut bytes[..];
    w.write_all(&30i32.to_le_bytes()).unwrap();
    for i in 0..15
    {
        let (off, len) = if i == LumpIdx::Entities as usize
        {
            (header_size as i32, 10i32)
        }
        else
        {
            (0i32, 0i32)
        };
        w.write_all(&off.to_le_bytes()).unwrap();
        w.write_all(&len.to_le_bytes()).unwrap();
    }
    bytes[header_size..].copy_from_slice(&[0xFF, 0xFE, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
    let path = tmp_path("non_utf8.bsp");
    fs::write(&path, &bytes)?;
    let bsp = BspFile::load(&path)?;
    let out = tmp_path("non_utf8_out.ent");
    let result = ent::extract(&bsp, &out, ExtractTarget::Single);
    assert!(result.is_err());
    let _ = fs::remove_file(&path);
    Ok(())
}

#[test]
fn test_extract_split_only_point_plus_worldspawn() -> Result<()>
{
    let entities = "{\n\"classname\" \"worldspawn\"\n\"wad\" \"test.wad\"\n}\n{\n\"classname\" \"light\"\n\"origin\" \"0 0 0\"\n}\n{\n\"classname\" \"info_player_deathmatch\"\n\"origin\" \"128 128 0\"\n}\n";
    let bsp_path = tmp_path("point_plus_world.bsp");
    write_synthetic_bsp(&bsp_path, entities);
    let bsp = BspFile::load(&bsp_path)?;
    let base = tmp_path("point_plus_world_result");
    ent::extract(&bsp, &base, ExtractTarget::Split)?;
    let point_text = fs::read_to_string(base.with_extension(EXT_POINT_ENT))?;
    let brush_text = fs::read_to_string(base.with_extension(EXT_BRUSH_ENT))?;
    assert!(point_text.contains("light"));
    assert!(point_text.contains("info_player_deathmatch"));
    assert!(!point_text.contains("worldspawn"));
    assert!(brush_text.contains("worldspawn"));
    assert!(!brush_text.contains("light"));
    assert!(!brush_text.contains("info_player_deathmatch"));
    let _ = fs::remove_file(&bsp_path);
    let _ = fs::remove_file(base.with_extension(EXT_POINT_ENT));
    let _ = fs::remove_file(base.with_extension(EXT_BRUSH_ENT));
    Ok(())
}

// ─── normalise_entities edge cases ───────────────────────────────────────────

#[test]
fn test_normalise_empty_input()
{
    assert_eq!(crate::bsp::ent::normalise_entities(""), "");
}

#[test]
fn test_normalise_negative_depth()
{
    let result = crate::bsp::ent::normalise_entities("}\n}\n}\n");
    assert!(!result.is_empty());
}

// ─── collect_bsps wildcard ───────────────────────────────────────────────────

#[test]
fn test_collect_bsps_wildcard()
{
    let dir = temp_dir("collect_bsps_wildcard");
    write_bsp_file(&dir.join("weapons_test.bsp"));
    write_bsp_file(&dir.join("weapons_extra.bsp"));
    write_bsp_file(&dir.join("npc_test.bsp"));
    let path = dir.join("weapons_*");
    let bsps = exec::collect_bsps(&path);
    assert_eq!(bsps.len(), 2);
    for b in &bsps
    {
        let name = b.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("weapons_"));
    }
}

#[test]
fn test_collect_bsps_wildcard_no_match()
{
    let dir = temp_dir("collect_bsps_wildcard_no_match");
    write_bsp_file(&dir.join("map1.bsp"));
    let path = dir.join("zx*");
    let bsps = exec::collect_bsps(&path);
    assert!(bsps.is_empty());
}

// ─── batch_stats ─────────────────────────────────────────────────────────────

#[test]
fn test_batch_stats_single_bsp() -> Result<()>
{
    let dir = temp_dir("batch_stats_single");
    let bsp_path = dir.join("map.bsp");
    let entities = "{\n\"classname\" \"worldspawn\"\n}\n{\n\"classname\" \"light\"\n\"origin\" \"0 0 0\"\n}\n";
    write_synthetic_bsp(&bsp_path, entities);
    let results = exec::batch_stats(&dir)?;
    assert_eq!(results.len(), 1);
    let (_path, report) = &results[0];
    assert!(report.contains("Total entities"));
    assert!(!report.contains("Failed to load"));
    let _ = fs::remove_file(&bsp_path);
    Ok(())
}

#[test]
fn test_batch_stats_load_failure() -> Result<()>
{
    let dir = temp_dir("batch_stats_fail");
    let bad_path = dir.join("corrupt.bsp");
    fs::write(&bad_path, &[0u8; 10])?;
    let results = exec::batch_stats(&dir)?;
    assert_eq!(results.len(), 1);
    let (_path, report) = &results[0];
    assert!(report.contains("Failed to load"));
    let _ = fs::remove_file(&bad_path);
    Ok(())
}

// ─── remove_files ────────────────────────────────────────────────────────────

#[test]
fn test_remove_files_empty_input()
{
    crate::utils::remove_files(&[], None);
    crate::utils::remove_files(&[], Some("ent"));
}

#[test]
fn test_remove_files_with_extension() -> Result<()>
{
    let dir = temp_dir("remove_files_ext");
    let p1 = dir.join("test.bsp");
    let p2 = dir.join("other.bsp");
    fs::write(&p1, "data")?;
    fs::write(&p2, "data")?;
    assert!(p1.exists());
    assert!(p2.exists());
    crate::utils::remove_files(&[p1.clone(), p2.clone()], Some("bsp"));
    assert!(!p1.exists());
    assert!(!p2.exists());
    let _ = fs::remove_dir_all(&dir);
    Ok(())
}
