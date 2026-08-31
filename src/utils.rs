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
/// Returns the current directory path, or "." if it fails.
#[macro_export] macro_rules! current_dir_path
{
    () =>
    {
        cfg_select!
        {
            windows => std::env::current_dir().unwrap_or( std::path::PathBuf::from( "." ) ),
            target_os = "linux" =>
            {
                std::env::current_exe()
                    .ok()
                    .and_then( |p| p.parent().map( |p| p.to_path_buf() ) )
                    .filter( |p| p.is_dir() )
                .unwrap_or_else( || std::env::current_dir().unwrap_or( std::path::PathBuf::from( "." ) ) )
            }
        }
    };
}

#[macro_export] macro_rules! clear_terminal
{
    () =>
    {
        {
            use crossterm::
            {
                cursor,
                execute,
                terminal::
                {
                    Clear,
                    ClearType
                }
            };

            let _ = execute!( std::io::stdout(), Clear( ClearType::All ), cursor::MoveTo( 0, 0 ) );
        }
    };
}
/// Gets CLI arguments (flags) and values
pub(crate) fn get_args<F, V>() -> (Box<[F]>, Box<[V]>)
where
    F: std::str::FromStr,
    V: std::str::FromStr + Default
{
    let args: Vec<_> = std::env::args().skip( 1 ).collect();

    if args.is_empty()
    {
        return ( Box::new( [] ), Box::new( [] ) );
    }

    let ( mut flags, mut values ) = ( vec![], vec![] );

    for a in args
    {
        if let Ok( f ) = a.parse()
        {   // Is a flag
            flags.push( f );
        }
        else// Is a value (file path)
        {
            values.push( a.parse::<V>().unwrap_or_default() )
        }
    }

    ( flags.into_boxed_slice(), values.into_boxed_slice() )
}

pub(crate) trait HasExtension
{
    fn has_extension(&self, extensions: &[&str]) -> bool;
}

impl<T: AsRef<std::path::Path>> HasExtension for T
{   /// Checks if a string has any of the specified extensions
    fn has_extension(&self, extensions: &[&str]) -> bool
    {
        let Some( ext ) = self.as_ref().extension().and_then( |e| e.to_str() )
        else
        {
            return false;
        };

        extensions.iter().any( |e| ext.eq_ignore_ascii_case( e ) )
    }
}

pub(crate) fn remove_files(paths: &[std::path::PathBuf], some_extension: Option<&str>)
{
    if paths.is_empty()
    {
        return;
    }

    for p in paths
    {
        if let Some( ext ) = some_extension
        {
            if let Err( e ) = std::fs::remove_file( p.with_extension( ext ) )
            {
                eprintln!( "⚠️ Couldn't delete '{p:?}' with extension '{ext}': {e}" );
            }
        }
        else
        {
            if let Err( e ) = std::fs::remove_file( p )
            {
                eprintln!( "⚠️  Couldn't delete {p:?}: {e}" );
            }
        }
    }
}
