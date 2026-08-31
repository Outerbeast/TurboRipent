/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.1.0

Copyright (C) 2025 Outerbeast
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
use std::
{
    collections::HashMap,
    fmt::Write,
    fs,
    path::
    {
        Path,
        PathBuf
    },
    ops::
    {
        Deref,
        DerefMut,
        Index
    }
};

use anyhow::
{
    Result,
    bail
};

use crossterm::style::Stylize;

use crate::prelude::*;

pub(crate) const EXT_ENT: &str = "ent";
pub(crate) const EXT_POINT_ENT: &str = "entp";
pub(crate) const EXT_BRUSH_ENT: &str = "entm";
pub(crate) const EXT_BSP: &str = "bsp";

#[derive( Clone, Default, Debug, PartialEq )]
pub(crate) struct EntityDictionary(HashMap<String, String>);

impl EntityDictionary
{   /// Constructor
    pub(crate) fn new(classname: &str) -> Self
    {
        Self( HashMap::from( [( "classname".to_string(), classname.to_string() )] ) )
    }
    /// Constructor
    fn from_ent_block(ent_block: &str) -> Self
    {
        let mut this = Self::default();
        let mut iter = ent_block.split( '"' ).skip( 1 ).step_by( 2 );

        while let Some( key ) = iter.next()
        {
            if let Some( value ) = iter.next()
            {
                this.insert( key.to_string(), value.to_string() );
            }
        }

        this
    }
    /// Collection constructor - entity plain text to edicts
    #[inline] pub(crate) fn from_ent_txt(ent_txt: &str) -> Vec<Self>
    {
        Self::make_iter( ent_txt ).collect()
    }
    /// Iterator constructor - entity plain text to edicts iterator
    pub(crate) fn make_iter(ent_txt: &str) -> impl Iterator<Item = Self>
    {
        ent_txt.split( '{' ).skip( 1 ).filter_map( |block|
        {
            let inner = block.split_once( '}' )?.0;
            Some( Self::from_ent_block( inner ) )
        })
    }

    pub(crate) fn get_classname(&self) -> &str
    {
        self.0
            .get( "classname" )
            .filter( |s| !s.is_empty() )
            .map( |s| s.as_str() )
        .unwrap_or( "<no classname>" )
    }
    /// Returns the brush model index for brush entities (worldspawn returns 0), or None for point entities.
    pub(crate) fn get_model_index(&self) -> Option<usize>
    {
        if self.get_classname() == "worldspawn"
        {
            return Some( 0 );
        }

        self.0
            .get( "model" )
            .and_then( |m| m.strip_prefix( '*' ) )
            .and_then( |s| s.parse().ok() )
    }
    /// Returns the spawnflags value for entities that have it, or None for entities that don't.
    #[inline] pub(crate) fn get_spawnflags(&self) -> Option<u32>
    {
        self.0.get( "spawnflags" ).and_then( |s| s.parse().ok() )
    }

    pub(crate) fn set_spawnflags(&mut self, flags: u32)
    {
        if flags == 0
        {
            self.0.remove( "spawnflags" );
            return;
        }

        self.0.insert( "spawnflags".to_string(), flags.to_string() );
    }

    pub(crate) fn from_kv_pairs(pairs: &[(&str, &str)] ) -> Self
    {
        let mut this = EntityDictionary::default();

        for row in pairs
        {
            let key = row.0.trim();

            if !key.is_empty()
            {
                this.insert( key.to_string(), row.1.to_string() );
            }
        }

        this
    }

    pub(crate) fn to_kv_pairs(&self) -> Vec<(String, String)>
    {
        let mut keys: Vec<_> = self.keys().map( |key| key.as_str() ).collect();
        keys.sort();

        keys
            .into_iter()
            .map( |key| ( key.to_string(), self[key].to_string() ) )
        .collect()
    }

    pub(crate) fn to_txt(entities: &[Self]) -> String
    {
        let mut buf = String::new();

        for kv in entities
        {
            if kv.is_empty()
            {
                continue;
            }

            buf.push_str( "{\n" );

            for (key, value) in kv.iter()
            {
                writeln!( buf, "\"{key}\" \"{value}\"" ).expect( "infallible: writing to String" );
            }

            buf.push_str( "}\n" );
        }

        buf
    }
    /// Loads entities from ENT or BSP files, returns a collection of edicts
    pub(crate) fn load_entities(file_path: &Path) -> Result<Vec<Self>>
    {
        match file_path.extension().and_then( |ext| ext.to_str() )
        {
            Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>
                Ok( Self::from_ent_txt( &fs::read_to_string( file_path )? ) ),
            
            Some( EXT_BSP ) =>
            {
                let bsp = BspFile::load( file_path )?;
                Ok( Self::from_ent_txt( &ExtractTarget::Text.with( &bsp, None )? ) )
            }

            Some( other_ext ) => bail!( "Invalid file type '{other_ext}'. Requires ENT/BSP." ),
            None => bail!( "Cannot use folder. Must specify a ENT/BSP file." )
        }
    }
    /// Saves a collection of entities to a file.
    /// For plain ENT text files, contents are overwritten and saved
    /// For BSP files, the entity data is imported into the BSP
    pub(crate) fn save_entities(entities: &[Self], file_path: &Path) -> Result<()>
    {
        let ent_txt = Self::to_txt( entities );

        match file_path.extension().and_then( |ostr| ostr.to_str() )
        {
            Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) => Ok( fs::write( file_path, &ent_txt )? ),
            Some( EXT_BSP ) => Ok( ImportSource::Text( ent_txt ).with( BspFile::load( file_path )? )?.save()? ),
            Some( other_ext ) => bail!( "Invalid file type '{other_ext}'. Requires ENT/BSP." ),
            None => bail!( "Cannot use folder. Must specify a ENT/BSP file." )
        }
    }
    /// Converts a JSON string into a vector of EntityDictionary objects.
    #[cfg( false )]
    pub fn from_json(json: &str) -> Result<Vec<Self>>
    {
        serde_json::from_str( json )?.iter().map( |v|
        {
            let map = v["KeyValues"].as_object().context( "missing KeyValues" )?;

            Ok( Self( map
                .iter()
                .map( |(k, v)| ( k.clone(), v.as_str().unwrap_or_default().to_string() ) )
                .collect() ) )
        })
        .collect()
    }
    /// Converts a vector of EntityDictionary objects into a JSON string.
    #[cfg( false )]
    pub fn to_json(entities: &[Self]) -> String
    {
        let arr: Vec<_> = entities.iter().map( |e| serde_json::json!({ "KeyValues": e.0 }) ).collect();
        serde_json::to_string_pretty( &arr ).unwrap()
    }
}

impl Deref for EntityDictionary
{
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for EntityDictionary
{
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Index<&str> for EntityDictionary
{
    type Output = String;

    fn index(&self, key: &str) -> &String { &self.0[key] }
}
pub(crate) enum ExtractTarget
{
    Text,
    Single,
    Split
}

impl ExtractTarget
{
    /// Extracts entity data from a BSP file.
    /// Returns the extracted data as a string.
    /// If a output path is specified, the extracted data will be written to that path as an:
    /// - ent file (Single)
    /// - entp + entm (Split)
    pub(crate) fn with(&self, bsp: &BspFile, some_output_path: Option<&Path>) -> Result<String>
    {
        let ent_data = bsp.slice_lump( LumpIdx::Entities );

        if ent_data.is_empty()
        {
            bail!( "Entity lump is empty" );
        }

        let all_ents =
        if let Some( &0 ) = ent_data.last()
        {
            str::from_utf8( &ent_data[..ent_data.len() - 1] )?.to_owned()
        }
        else
        {
            str::from_utf8( ent_data )?.to_owned()
        };

        match self
        {
            Self::Text => Ok( all_ents ),
            Self::Single =>
            {
                let Some( out ) = some_output_path 
                else
                {
                    bail!( "ExtractTarget::Single requires an output path" );
                };

                fs::write( out, all_ents.as_bytes() )?;
                Ok( all_ents )
            }

            Self::Split =>
            {
                let Some( out ) = some_output_path 
                else
                {
                    bail!( "ExtractTarget::Split requires an output path" );
                };
                
                let( mut point_ents, mut brush_ents ) = ( String::new(), String::new() );

                for ent in EntityDictionary::make_iter( &all_ents )
                {
                    let is_brush = ent.get_model_index().is_some();
                    let line = EntityDictionary::to_txt( &[ent] );

                    if is_brush
                    {
                        brush_ents.push_str( &line );
                    }
                    else
                    {
                        point_ents.push_str( &line );
                    }
                }

                fs::write( out.with_extension( EXT_POINT_ENT ), point_ents.as_bytes() )?;
                fs::write( out.with_extension( EXT_BRUSH_ENT ), brush_ents.as_bytes() )?;

                println!( "Extracted split entities → {:?} and {:?}",
                    out.with_extension( EXT_POINT_ENT ), out.with_extension( EXT_BRUSH_ENT ) );

                Ok( all_ents )
            }
        }
    }
}

pub(crate) enum ImportSource
{
    Text(String),
    Single(PathBuf),
    Split(PathBuf)
}

impl ImportSource
{
    /// Imports entity data from an entity file into a BSP.
    /// The BSP file is NOT ovewritten here - BspFile::save() must be executed after this function returns OK.
    /// This avoids BSP corruption.
    pub(crate) fn with(self, mut bsp: BspFile) -> Result<BspFile>
    {
        let ent_txt = 
        match self
        {
            Self::Text( t ) => t,
            Self::Single( p ) => fs::read_to_string( p )?,
            Self::Split( base ) => // Content has to be combined before importing
            {
                let point_ents_file = base.with_extension( EXT_POINT_ENT );
                let brush_ents_file = base.with_extension( EXT_BRUSH_ENT );

                if !point_ents_file.try_exists().unwrap_or( false )
                {
                    bail!( "Cannot import split entities: missing {point_ents_file:?}.\n
                        Both .{EXT_POINT_ENT} and .{EXT_BRUSH_ENT} are required." );
                }

                if !brush_ents_file.try_exists().unwrap_or( false )
                {
                    bail!( "Cannot import split entities: missing {brush_ents_file:?}.\n
                        Both .{EXT_POINT_ENT} and .{EXT_BRUSH_ENT} are required." );
                }

                let mut combined = fs::read_to_string( &point_ents_file )?;
                combined.push( '\n' );
                combined.push_str( &fs::read_to_string( &brush_ents_file )? );

                combined
            }
        };

        let mut entities = EntityDictionary::from_ent_txt( &normalise_entities( &ent_txt ) );
        strip_invalid_brushents( &mut entities, &bsp );
        let mut out = EntityDictionary::to_txt( &entities );

        if !out.ends_with( '\0' )
        {
            out.push( '\0' );
        }

        bsp.replace_lump( LumpIdx::Entities, out.as_bytes() )?;

        Ok( bsp )
    }
}
/// Normalises the entity text to a consistent format, fixing issues with brace nesting and alignment.
pub(crate) fn normalise_entities(text: &str) -> String
{   // Pass 1: Structural normalisation (quote-aware brace handling)
    let mut struct_fixed = String::new();
    let mut in_quote = false;
    let mut depth = 0;

    for c in text.chars()
    {
        match c
        {
            '\r' => continue,
            '"' =>
            { 
                in_quote = !in_quote;
                struct_fixed.push( c );
            }

            '{' if !in_quote =>
            {
                if !struct_fixed.is_empty() && !struct_fixed.ends_with( '\n' )
                {
                    struct_fixed.push( '\n' );
                }

                struct_fixed.push_str( "{\n" );
                depth = depth.max( 0 ) + 1;
            }

            '}' if !in_quote =>
            {
                if !struct_fixed.ends_with( '\n' )
                {
                    struct_fixed.push( '\n' );
                }

                depth -= 1;
                struct_fixed.push( c );
                struct_fixed.push( '\n' );
            }

            _ if depth > 0 => struct_fixed.push( c ),
            _ => { }
        }
    }

    while depth > 0
    {
        struct_fixed.push_str( "}\n" );
        depth -= 1;
    }
    // Pass 2: Per-line kvp fixing
    let mut fixed = String::new();
    let mut in_block = false;

    for line in struct_fixed.lines()
    {
        let trimmed = line.trim();

        if trimmed == "{"
        {
            in_block = true;
            fixed.push_str( line );
            fixed.push( '\n' );
        }
        else if trimmed == "}"
        { 
            in_block = false;
            fixed.push_str( line );
            fixed.push( '\n' );
        }
        else if in_block && !trimmed.is_empty()
        {
            let q: Vec<_> = trimmed.split( '"' ).collect();
            match q.len() - 1
            {
                2 =>
                {
                    let key = q[1];
                    if !q[2].trim().is_empty()
                    {
                        writeln!( fixed, "\"{key}\" \"{}\"", q[2].trim() ).expect( "infallible: writing to String" );
                    }
                }

                3 => writeln!( fixed, "\"{}\" \"{}\"", q[1], q[3] ).expect( "infallible: writing to String" ),
                4 =>
                {
                    fixed.push_str( line );
                    fixed.push( '\n' );
                }

                5 ..= 7 =>
                {
                    let clean: String = q[3..q.len()-1].concat().chars().filter( |c| *c != '"' ).collect();
                    if !clean.is_empty()
                    {
                        writeln!( fixed, "\"{}\" \"{clean}\"", q[1] ).expect( "infallible: writing to String" );
                    }
                }

                _ =>
                {
                    let mut i = 0;
                    while i + 3 < q.len() - 1
                    {
                        writeln!( fixed, "\"{}\" \"{}\"", q[i + 1], q[i + 3] ).expect( "infallible: writing to String" );
                        i += 4;
                    }
                }
            }
        }
        else
        {
            fixed.push_str( line );
            fixed.push( '\n' );
        }
    }

    fixed
}
/// Strip out entities that reference non-existent brush models
fn strip_invalid_brushents(entities: &mut Vec<EntityDictionary>, bsp: &BspFile)
{
    let model_indices = bsp.slice_lump( LumpIdx::Models ).len() / 64;
    entities.retain( |ent|
    {
        let Some( idx ) = ent.get_model_index()
        else
        {
            return true
        };

        if idx == 0 || idx < model_indices
        {
            return true;
        }

        eprintln!( "⚠️ {}", format!( "entity \"{}\" references non-existent brush model *{idx}, discarding...", ent.get_classname() ).yellow() );

        false
    });
}
/// Runs entity normalisation operations on a BSP or ENT file.
pub(crate) fn repair(bsporent_path: &Path) -> Result<()>
{
    match bsporent_path.extension().and_then( |ext| ext.to_str() )
    {
        Some( EXT_BSP ) =>
        {
            let bsp = BspFile::load( bsporent_path )?;
            let ent_txt = normalise_entities( &ExtractTarget::Text.with( &bsp, None )? );
            ImportSource::Text( ent_txt ).with( bsp )?.save()?;
            println!( "Repaired entities → {bsporent_path:?}" );
        }

        Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>
        {
            fs::write( bsporent_path, normalise_entities( &fs::read_to_string( bsporent_path )? ).as_bytes() )?;
            println!( "Repaired entities → {bsporent_path:?}" );
        }

        Some( other_ext ) => bail!( "Invalid file type '{other_ext}'. Requires ENT/BSP." ),
        None => bail!( "Cannot use folder. Must specify a ENT/BSP file." )
    }

    Ok( () )
}
/// Decides automatically extraction/importation based on file type
pub(crate) fn rip(bsporent_path: &Path) -> Result<()>
{
    if !bsporent_path.exists() 
    {
        bail!( "BSP rip: '{:?}' does not exist.", bsporent_path );
    }

    match bsporent_path.extension().and_then( |ext| ext.to_str() )
    {
        Some( EXT_BSP ) =>// Extract from BSP to ENT
        {
            let ent_path = bsporent_path.with_extension( EXT_ENT );// "level.bsp" -> "level.ent"
            ExtractTarget::Single.with( &BspFile::load( bsporent_path )?, Some( &ent_path ) )?;
            println!( "Extracted entities → {ent_path:?}" );

            Ok( () )
        }

        Some( EXT_ENT ) =>// Import from ENT to BSP
        {
            let bsp_path = bsporent_path.with_extension( EXT_BSP );// "level.ent" -> "level.bsp"
            ImportSource::Single( bsporent_path.to_path_buf() ).with( BspFile::load( &bsp_path )? )?.save()?;
            println!( "Imported entities → {bsp_path:?}" );
            fs::remove_file( bsporent_path )?;

            Ok( () )
        }

        Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>// Import from .entp + .entm to BSP
        {
            let bsp_path = bsporent_path.with_extension( EXT_BSP );
            ImportSource::Split( bsp_path.with_extension( "" ) ).with( BspFile::load( &bsp_path )? )?.save()?;
            println!( "Imported split entities → {bsp_path:?}" );

            fs::remove_file( bsp_path.with_extension( EXT_POINT_ENT ) )?;
            fs::remove_file( bsp_path.with_extension( EXT_BRUSH_ENT ) )?;

            Ok( () )
        }

        Some( other_ext ) => bail!( "Invalid file type '{other_ext}'. Requires ENT/BSP." ),
        None => bail!( "Cannot use folder. Must specify a ENT/BSP file." )
    }
}
