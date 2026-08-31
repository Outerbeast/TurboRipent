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
    fmt::
    {
        Display,
        Formatter,
        Result
    },
    path::PathBuf
};

use crate::prelude::*;

pub(crate) struct EntityReport
{
    pub(crate) path: PathBuf,
    pub(crate) total_entities: usize,
    pub(crate) point_entities: usize,
    pub(crate) brush_entities: usize,
    pub(crate) total_brush_models: usize,
    pub(crate) unused_model_indices: Vec<usize>
}

impl EntityReport
{
    pub(crate) fn generate(bsp: &BspFile) -> Self
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
        let entities = EntityDictionary::from_ent_txt( all_ents );

        let mut point = 0;
        let mut brush = 0;
        let mut referenced = vec![false; model_count];
        referenced[0] = true;// Worldspawn HAS to exist

        for ent in &entities
        {
            if ent.get_model_index().is_some()
            {
                brush += 1;
            }
            else
            {
                point += 1;
            }

            if let Some( idx ) = ent.get_model_index()
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
            unused_model_indices: ( 1..model_count ).filter( |&i| !referenced[i] ).collect()
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
