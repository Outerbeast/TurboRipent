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
fn main() -> std::io::Result<()>
{
    const PRODUCT_NAME: &str = env!( "CARGO_PKG_NAME" );
    let root = std::path::PathBuf::from( std::env::var_os( "CARGO_MANIFEST_DIR" )
        .unwrap_or_default() );

    cfg_select!
    {
        windows =>
        {
            const AUTHOR: &str = env!( "CARGO_PKG_AUTHORS" );
            const VERSION: &str = env!( "CARGO_PKG_VERSION" );
            const DESCRIPTION: &str = env!( "CARGO_PKG_DESCRIPTION" );

            winresource::WindowsResource::new()
                .set( "ProductName", PRODUCT_NAME )
                .set( "ProductVersion", VERSION )
                .set( "FileDescription", DESCRIPTION )
                .set( "FileVersion", VERSION )
                .set( "LegalCopyright", AUTHOR )
                .set( "OriginalFilename", &format!( "{PRODUCT_NAME}.exe" ) )
                .set( "InternalName", PRODUCT_NAME )
                .set( "CompanyName", AUTHOR )
                .set( "LegalTrademarks", AUTHOR )
                .set( "Comments", DESCRIPTION )
                .set_icon( concat!( env!( "CARGO_PKG_NAME" ), ".ico" ) ) 
                .set_manifest( include_str!( concat!( env!( "CARGO_PKG_NAME" ), ".manifest.xml" ) ) )
            .compile()?;

            let cmd = format!( "@echo off\n\"%~dp0{PRODUCT_NAME}.exe\" -edit \"%~1\"" );
            std::fs::write( root.join( format!( "{PRODUCT_NAME}-Editor.cmd" ) ), cmd )
        }

        unix =>
        {
            use std::os::unix::fs::PermissionsExt;

            let sh = format!(
                "#!/bin/sh\n\
                script_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\n\
                exec \"$script_dir/{PRODUCT_NAME}\" -edit \"$@\"\n" );

            let editor_script = root.join( format!( "{PRODUCT_NAME}-Editor.sh" ) );
            std::fs::write( &editor_script, sh )?;
            let mut permissions = std::fs::metadata( &editor_script )?.permissions();
            permissions.set_mode( 0o755 );
            std::fs::set_permissions( &editor_script, permissions )?;

            let desktop = format!(
                "[Desktop Entry]\n\
                Name={PRODUCT_NAME} Editor\n\
                Comment=Edit BSP and ENT files\n\
                Exec=\"{}\" %f\n\
                Terminal=true\n\
                Type=Application\n\
                Categories=Utility;\n",
                editor_script.display() );

            std::fs::write( root.join( format!( "{PRODUCT_NAME}-Editor.desktop" ) ), desktop )
        }

    } 
}