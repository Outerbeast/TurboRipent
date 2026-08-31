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
pub(crate) mod view;
pub(crate) mod controller;

use std::path::
{
    Path,
    PathBuf
};

use anyhow::
{
    anyhow,
    Result,
    bail
};

use super::
{
    cli::get_prompt_input,
    bsp::ent::EntityDictionary
};

use crate::prelude::*;
/// Launch the application with the given to BSP or ENT
pub(crate) fn launch(chosen_path: impl AsRef<Path>) -> Result<()>
{
    let chosen_path = chosen_path.as_ref();
    let edit_path =
    if chosen_path.to_string_lossy().is_empty()
    || !chosen_path.has_extension( &[EXT_BSP, EXT_ENT, EXT_POINT_ENT, EXT_BRUSH_ENT] )
    {
        let path_str =
        get_prompt_input( "Drag a BSP or ENT file you want to edit (enter 'x' to cancel):" );

        if path_str.is_empty() || path_str == "x"
        {
            bail!( "User cancelled." );
        }

        PathBuf::from( path_str )
    }
    else
    {
        PathBuf::from( chosen_path )
    };

    println!( "Opening: {edit_path:?}" );
    // Launch the TUI
    view::display( &edit_path, &EntityDictionary::load_entities( &edit_path )? )
        .map_err( |e| anyhow!( e.to_string() ) )
}
