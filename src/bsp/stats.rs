/*
	TurboRipent - TUI Frontend for Ripent
	Version 2.0

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
    fmt::
    {
        Display,
        Formatter,
        Result
    },
    path::PathBuf
};

use crate::
{
    bsp::
    {
        BspFile,
        LumpIdx,
        ent::
        {
            is_brush_ent,
            parse_entity_blocks
        }
    }
};

pub struct EntityReport
{
    pub path: PathBuf,
    pub total_entities: usize,
    pub point_entities: usize,
    pub brush_entities: usize,
    pub total_brush_models: usize,
    pub unused_model_indices: Vec<usize>
}

impl EntityReport
{
    pub fn generate(bsp: &BspFile) -> Self
    {
        let ent_data = bsp.slice_lump( LumpIdx::Entities );
        let model_count = bsp.slice_lump( LumpIdx::Models ).len() / 64;
        
        let all_ents =
        if let Some( &0 ) = ent_data.last()
        {
            &ent_data[..ent_data.len() - 1]
        }
        else
        {
            ent_data
        };

        let all_ents = str::from_utf8( all_ents ).unwrap_or( "" );
        let entities: Vec<_> = parse_entity_blocks( all_ents )
            .into_iter().map( |(_, d)| d ).collect();

        let mut point: usize = 0;
        let mut brush: usize = 0;
        let mut referenced = vec![false; model_count];
        referenced[0] = true;// Worldspawn HAS to exist

        for ent in &entities
        {
            if is_brush_ent( ent )
            {
                brush += 1;
            }
            else
            {
                point += 1;
            }

            if let Some( model_val ) = ent.get( "model" )
            && let Some( idx_str ) = model_val.strip_prefix( '*' )
            && let Ok( idx ) = idx_str.parse::<usize>()
            && idx < model_count
            {
                referenced[idx] = true;
            }
        }

        Self
        {
            path: bsp.path.clone(),
            total_entities: entities.len(),
            point_entities: point,
            brush_entities: brush,
            total_brush_models: model_count,
            unused_model_indices: ( 1..model_count ).filter( |&i| !referenced[i] ).collect(),
        }
    }
}
// Formats the report as a big fat string 
impl Display for EntityReport
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result
    {
        writeln!( f, "  File: {:?}", self.path )?;
        writeln!( f )?;
        writeln!( f, "  Point entities:         {:>5}", self.point_entities )?;
        writeln!( f, "  Brush entities:         {:>5}", self.brush_entities )?;
        writeln!( f, "  Total entities:         {:>5}", self.total_entities )?;
        writeln!( f, "  Brush models in lump:   {:>5}", self.total_brush_models )?;
        writeln!( f )?;

        if self.unused_model_indices.is_empty()
        {
            writeln!( f, "  [Unused brush models]  0" )?;
            writeln!( f, "    (none)" )?;
        }
        else
        {
            writeln!( f, "  [Unused brush models]  {}", self.unused_model_indices.len() )?;
            for idx in &self.unused_model_indices
            {
                writeln!( f, "    model *{} exists but no entity references it", idx )?;
            }
        }

        writeln!( f )
    }
}
