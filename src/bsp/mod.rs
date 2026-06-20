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
pub(crate) mod ent;
pub(crate) mod stats;

use std::
{
    fs,
    io::
    {
        self
    },
    ops,
    path::
    {
        Path,
        PathBuf
    }
};

use anyhow::
{
    Error,
    Result,
    anyhow,
    bail
};

use strum::EnumCount;
use strum_macros::EnumCount;

const VERSION: i32 = 0x1E;// 30
const HEADER_SIZE: usize = ( 8 * LumpIdx::COUNT ) + 4;
const LUMP_ENTRY_SIZE: usize = 8;

#[repr( usize )]
#[derive( Debug, Clone, Copy, EnumCount )]
pub enum LumpIdx
{
    Entities,
    Planes,
    Textures,
    Vertices,
    Visibility,
    Nodes,
    TexInfo,
    Faces,
    Lighting,
    ClipNodes,
    Leaves,
    MarkSurfaces,
    Edges,
    SurfEdges,
    Models
}

impl From<LumpIdx> for usize
{
    fn from(l: LumpIdx) -> usize { l as usize }
}
// Represents a single lump in the BSP file defined by its start offset and size.
#[derive( Debug, Clone, Copy )]
pub struct Lump(pub i32, pub i32);// (start, length)

impl Lump
{
    pub fn length(&self) -> i32
    {
        self.1
    }

    pub fn range(&self) -> ops::Range<usize>
    {
        let start = self.0 as usize;
        let end = start + self.1 as usize;

        start..end
    }
}

#[derive( Debug, Clone )]
pub struct BspHeader// BSP header - contains the offsets and sizes of all lumps in the file.
{
    pub version: i32,
    pub lumps: [Lump; LumpIdx::COUNT]
}

impl BspHeader
{
    pub fn lump(&self, idx: LumpIdx) -> Lump
    {
        self.lumps[idx as usize]
    }

    pub fn write_to(&self, buf: &mut [u8]) -> Result<()>
    {
        use std::io::Write;
        let mut buf = buf;
        buf.write_all( &self.version.to_le_bytes() )?;
        for &Lump( offset, length ) in &self.lumps
        {
            buf.write_all( &offset.to_le_bytes() )?;
            buf.write_all( &length.to_le_bytes() )?;
        }

        Ok( () )
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self>
    {
        if data.len() < HEADER_SIZE
        {
            bail!( "File size doesn't match expected BSP version".to_string() );
        }

        let version = i32::from_le_bytes( data.get( 0..4 )
            .ok_or_else( || anyhow!( "Missing BSP version" ) )?
            .try_into()
        .map_err( |e| anyhow!( "Invalid BSP version bytes: {}", e ) )? );

        if version != VERSION
        {
            bail!( "Unsupported BSP version: {}. Requires BSP version: {}.", version, VERSION );
        }

        let mut lumps = [Lump(0, 0); LumpIdx::COUNT];
        let mut cursor = LUMP_ENTRY_SIZE / 2;

        for lump in lumps.iter_mut().take( LumpIdx::COUNT )
        {
            let offset = i32::from_le_bytes( data.get( cursor..cursor + 4 )
                .ok_or_else( || anyhow!( "Malformed BSP header" ) )?
                .try_into()
            .map_err( |e| anyhow!( "Malformed BSP header (offset): {}", e ) )? );

            let length = i32::from_le_bytes( data.get( cursor + 4..cursor + 8 )
                .ok_or_else( || anyhow!( "Malformed BSP header" ) )?
                .try_into()
            .map_err( |e| anyhow!( "Malformed BSP header (length): {}", e ) )? );

            *lump = Lump( offset, length );
            cursor += LUMP_ENTRY_SIZE;// Move to the next entry
        }

        Ok( Self { version, lumps } )
    }
}
// BSP file as a whole with header, content and path
pub struct BspFile
{
    pub header: BspHeader,
    pub content: Vec<u8>,
    pub path: PathBuf
}

impl BspFile
{
    pub fn load(path: &Path) -> Result<Self>
    {
        let buf = fs::read( path )?;
        let header = BspHeader::from_bytes( &buf )?;
        // Validate lump bounds
        for (i, lump) in header.lumps.iter().enumerate()
        {   // Check if the lump's end is beyond the buffer's end
            if lump.range().end > buf.len()
            {
                bail!( "Lump {} out of bounds", i );
            }
        }

        Ok( Self { content: buf, header, path: path.to_path_buf() } )
    }

    pub fn slice_lump(&self, idx: LumpIdx) -> &[u8]
    {
        &self.content[self.header.lump( idx ).range()]
    }

    pub fn replace_lump(&mut self, idx: LumpIdx, new_data: &[u8]) -> Result<(), Error>
    {
        let idx = usize::from( idx );
        // Update length
        self.header.lumps[idx].1 = new_data.len() as i32;
        // New BSP buffer with header reserved
        let mut new_bsp = vec![0; HEADER_SIZE];
        let mut cursor = HEADER_SIZE as i32;

        for i in 0..LumpIdx::COUNT
        {
            let lump = self.header.lumps[i];
            // Update offset
            self.header.lumps[i].0 = cursor;

            if i == idx
            {
                new_bsp.extend_from_slice( new_data );
                cursor += new_data.len() as i32;
            }
            else
            {
                let range = lump.range();
                new_bsp.extend_from_slice( &self.content[range] );
                cursor += lump.length();
            }
        }

        self.header.write_to( &mut new_bsp[..] )?;
        self.content = new_bsp;

        Ok( () )
    }

    pub fn save(&self) -> io::Result<()>
    {
        fs::write( &self.path, &self.content )
    }
}
