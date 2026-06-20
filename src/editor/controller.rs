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
    cell::RefCell,
    collections::HashMap,
    fs
};

use crossterm::style::Stylize;
use native_windows_gui::
{
    Event,
    EventData,
    MessageButtons,
    MessageIcons,
    MessageParams,
    bind_event_handler,
    modal_message,
    stop_thread_dispatch
};

use anyhow::Result;

use super::
{
    Dictionary,
    view::EditorWindow
};

use crate::bsp::
{
    ent,
    BspFile
};

thread_local!
{
    pub static ENTITIES: RefCell<Vec<Dictionary>> = const { RefCell::new( vec![] ) };
    static FILTERED_IDXS: RefCell<Vec<usize>> = const { RefCell::new( vec![] ) };
    static PREV_SEL: RefCell<i32> = const { RefCell::new( -1 ) };
    static UPDATING_LISTBOX: RefCell<bool> = const { RefCell::new( false ) };
}

fn save_entities(gui: &EditorWindow, entities: &[Dictionary]) -> Result<()>
{
    use crate::bsp::ent::{EXT_BRUSH_ENT, EXT_ENT, EXT_POINT_ENT};
    let text = ent::serialize_entities( entities );

    match gui.file_path.extension().and_then( |ostr| ostr.to_str() )
    {
        Some( EXT_ENT ) | Some( EXT_POINT_ENT ) | Some( EXT_BRUSH_ENT ) =>
        {
            fs::write( &gui.file_path, &text )?;
        }

        _ =>
        {
            fs::write( gui.file_path.with_extension( EXT_ENT ), &text )?;
            ent::import( BspFile::load( &gui.file_path )?, ent::ImportSource::Text( text ) )?.save()?;
        }
    }

    Ok( () )
}
// Collection of entities by classname (used for the entity list)
pub(crate) fn classnames_from_entities(entities: &[Dictionary]) -> Vec<String>
{
    entities
        .iter()
        .map( |kv|
        {
            kv.get( "classname" )
                .cloned()
                .filter( |s| !s.is_empty() )
            .unwrap_or_else( || "<no classname>".to_string() )
        })
    .collect()
}

pub(crate) fn parse_key_values(s: &str) -> Dictionary
{
    let mut kvs = Dictionary::new();

    for line in s.lines()
    {
        let line = line.trim();
        if line.is_empty()
        {
            continue;
        }

        if let Some( eq_pos ) = line.find( '=' )
        {
            let key = line[..eq_pos].trim().to_string();
            let val = line[eq_pos + 1..].trim().to_string();
            kvs.insert( key, val );
        }
    }

    kvs
}
// Function for rendering key-value pairs
pub(crate) fn render_key_values(kvs: &Dictionary) -> String
{
    if kvs.is_empty()
    {
        return String::new();
    }

    let mut keys: Vec<_> = kvs.keys().collect();
    keys.sort();
    let mut buf = String::new();
    
    for (i, k) in keys.iter().enumerate()
    {
        buf.push_str( &format!( "{k}={}", kvs[*k] ) );

        if i < keys.len() - 1
        {
            buf.push_str( "\r\n" );
        }
    }

    buf
}

pub fn populate_listbox(gui: &EditorWindow)
{
    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );
    gui.list.clear();
    FILTERED_IDXS.with( |f| f.borrow_mut().clear() );

    ENTITIES.with( |ent|
    {
        let entities = ent.borrow();
        let names = classnames_from_entities( &entities );

        for (i, name) in names.iter().enumerate()
        {
            gui.list.push( name.clone() );
            FILTERED_IDXS.with( |f| f.borrow_mut().push( i ) );
        }
    });

    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );

    if !FILTERED_IDXS.with( |f| f.borrow().is_empty() )
    {
        gui.list.set_selection( Some( 0 ) );
        if let Some( sel ) = gui.list.selection()
        {
            FILTERED_IDXS.with( |fi|
            {
                if let filtered = fi.borrow() && sel < filtered.len()
                {
                    let idx = filtered[sel];
                    ENTITIES.with( |e|
                    {
                        if let entities = e.borrow() && idx < entities.len()
                        {
                            gui.text.set_text( &render_key_values( &entities[idx] ) );
                        }
                    });
                }
            });
        }

        PREV_SEL.with( |p| *p.borrow_mut() = 0 );
    }
}
// Updates the listbox automatically when the filter changes
fn apply_filter(gui: &EditorWindow)
{
    let filter = gui.filter.text().trim().to_lowercase();
    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );
    gui.list.clear();
    FILTERED_IDXS.with(|f| f.borrow_mut().clear() );

    ENTITIES.with( |ent|
    {
        let entities = ent.borrow();

        if filter.is_empty()
        {
            for (i, name) in classnames_from_entities( &entities ).iter().enumerate()
            {
                gui.list.push( name.clone() );
                FILTERED_IDXS.with( |f| f.borrow_mut().push( i ) );
            }
        }
        else
        {
            for (i, ent) in entities.iter().enumerate()
            {
                for (k, v) in ent
                {
                    if k.to_lowercase().contains( &filter ) || v.to_lowercase().contains( &filter )
                    {
                        let class = ent
                            .get( "classname" )
                            .cloned()
                            .filter( |s| !s.is_empty() )
                        .unwrap_or_else( || "<no classname>".to_string() );

                        gui.list.push( class );
                        FILTERED_IDXS.with( |f| f.borrow_mut().push( i ) );

                        break;
                    }
                }
            }
        }
    });

    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );

    if !FILTERED_IDXS.with( |f| f.borrow().is_empty() )
    {
        gui.list.set_selection( Some( 0 ) );

        if let Some( sel ) = gui.list.selection()
        {
            FILTERED_IDXS.with( |fi|
            {
                if let filtered = fi.borrow() && sel < filtered.len()
                {
                    let idx = filtered[sel];
                    ENTITIES.with( |ent|
                    {
                        if let entities = ent.borrow() && idx < entities.len()
                        {
                            gui.text.set_text( &render_key_values( &entities[idx] ) );
                        }
                    });
                }
            });
        }

        PREV_SEL.with( |p| *p.borrow_mut() = 0 );
    }
    else
    {
        gui.list.set_selection( None );
        gui.text.set_text( "" );
        PREV_SEL.with( |p| *p.borrow_mut() = -1 );
    }
}

fn on_list_select(gui: &EditorWindow)
{
    if UPDATING_LISTBOX.with( |u| *u.borrow() )
    {
        return;
    }

    if let Some( sel ) = gui.list.selection()
    {
        FILTERED_IDXS.with( |f|
        {
            if let filtered = f.borrow() && sel < filtered.len()
            {
                let idx = filtered[sel];

                PREV_SEL.with( |p|
                {
                    if let prev = *p.borrow() && prev >= 0
                    {
                        ENTITIES.with( |ent|
                        {
                            let mut entities = ent.borrow_mut();
                            if ( prev as usize ) < entities.len()
                            {
                                entities[prev as usize] = parse_key_values( &gui.text.text() );
                            }
                        });
                    }
                });

                ENTITIES.with( |ent|
                {
                    if let entities = ent.borrow() && idx < entities.len()
                    {
                        gui.text.set_text( &render_key_values( &entities[idx] ) );
                    }
                });

                PREV_SEL.with( |p| *p.borrow_mut() = idx as i32 );
            }
        });
    }
}

fn on_text_change(gui: &EditorWindow)
{
    if UPDATING_LISTBOX.with( |u| *u.borrow() )
    {
        return;
    }

    if let Some( sel ) = gui.list.selection()
    {
        FILTERED_IDXS.with( |f|
        {
            if let filtered = f.borrow() && sel < filtered.len()
            {
                let idx = filtered[sel];
                ENTITIES.with( |ent|
                {
                    if let mut entities = ent.borrow_mut() && idx < entities.len()
                    {
                        entities[idx] = parse_key_values( &gui.text.text() );
                    }
                });

                refresh_listbox_item( gui, sel as i32 );
            }
        });
    }
}

fn refresh_listbox_item(gui: &EditorWindow, sel: i32)
{
    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );

    FILTERED_IDXS.with( |f|
    {
        if let filtered = f.borrow() && sel >= 0 && (sel as usize) < filtered.len()
        {
            let idx = filtered[sel as usize];
            ENTITIES.with( |ent|
            {
                if let entities = ent.borrow() && idx < entities.len()
                {
                    let class = entities[idx]
                        .get( "classname" )
                        .cloned()
                        .filter( |s| !s.is_empty() )
                    .unwrap_or_else( || "<no classname>".to_string() );
                    // Update the listbox item text
                    let new_collection =
                    {
                        let collection = gui.list.collection();
                        if (sel as usize) < collection.len()
                        {
                            let mut new_collection = collection.clone();
                            new_collection[sel as usize] = class;
                            Some( new_collection )
                        }
                        else
                        {
                            None
                        }
                    };
                    if let Some( new_collection ) = new_collection
                    {
                        gui.list.set_collection( new_collection );
                        gui.list.set_selection( Some( sel as usize ) );
                    }
                }
            });
        }
    });

    UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );
}

fn on_create(gui: &EditorWindow)
{
    let new_entity = HashMap::from( [( "classname".to_string(), "new_entity".to_string() )] );
    ENTITIES.with( |ent|
    {
        let mut entities = ent.borrow_mut();
        entities.push( new_entity );
        let idx = entities.len() - 1;

        let name = entities[idx]
            .get( "classname" )
            .cloned()
            .filter( |s| !s.is_empty() )
        .unwrap_or_else( || "<no classname>".to_string() );

        UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );
        gui.list.push( name );
        FILTERED_IDXS.with( |f| f.borrow_mut().push( idx ) );
        gui.list.set_selection( Some( gui.list.len() - 1 ) );
        UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );
        PREV_SEL.with( |p| *p.borrow_mut() = idx as i32 );
        gui.text.set_text( &render_key_values(&entities[idx] ) );
    });

    save_entities( gui, &ENTITIES.with( |ent| ent.borrow().clone() ) ).ok();
}

fn on_clone(gui: &EditorWindow)
{
    if let Some( sel ) = gui.list.selection()
    {
        let idx = FILTERED_IDXS.with( |f|
        {
            if let filtered = f.borrow() && sel < filtered.len()
            {
                Some( filtered[sel] )
            }
            else
            {
                None
            }
        });

        let Some( idx ) = idx else { return; };

        ENTITIES.with( |ent|
        {
            if let mut entities = ent.borrow_mut() && idx < entities.len()
            {
                let cloned = entities[idx].clone();
                entities.push( cloned );
                let new_idx = entities.len() - 1;

                let name = entities[new_idx]
                    .get( "classname" )
                    .cloned()
                    .filter( |s| !s.is_empty() )
                .unwrap_or_else( || "<no classname>".to_string() );

                UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );
                gui.list.push( name );
                FILTERED_IDXS.with( |f| f.borrow_mut().push( new_idx ) );
                gui.list.set_selection( Some( gui.list.len() - 1 ) );
                UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );

                PREV_SEL.with( |p| *p.borrow_mut() = new_idx as i32 );
                gui.text.set_text( &render_key_values( &entities[new_idx] ) );
            }
        });

        save_entities( gui, &ENTITIES.with( |ent| ent.borrow().clone() ) ).ok();
    }
}

fn on_delete(gui: &EditorWindow)
{
    if let Some( sel ) = gui.list.selection()
    {
        let idx = FILTERED_IDXS.with( |f|
        {
            if let filtered = f.borrow() && sel < filtered.len()
            {
                Some( filtered[sel] )
            }
            else
            {
                None
            }
        });

        let Some( idx ) = idx else { return; };

        ENTITIES.with( |ent|
        {
            if let mut entities = ent.borrow_mut() && idx < entities.len()
            {
                entities.remove( idx );
                UPDATING_LISTBOX.with( |u| *u.borrow_mut() = true );
                gui.list.remove( sel );
                FILTERED_IDXS.with( |f| f.borrow_mut().remove( sel ) );
                let new_len = gui.list.len();

                if sel < new_len
                {
                    gui.list.set_selection( Some( sel ) );
                    if let Some( new_sel ) = gui.list.selection()
                    {
                        let new_idx = FILTERED_IDXS.with( |fi|
                        {
                            if let filtered = fi.borrow() && new_sel < filtered.len()
                            {
                                Some( filtered[new_sel] )
                            }
                            else
                            {
                                None
                            }
                        });

                        if let Some( new_idx ) = new_idx
                        {
                            gui.text.set_text( &render_key_values( &entities[new_idx] ) );
                        }
                    }
                }
                else if new_len > 0
                {
                    gui.list.set_selection( Some( new_len - 1 ) );
                    if let Some( new_sel ) = gui.list.selection()
                    {
                        let new_idx = FILTERED_IDXS.with( |fi|
                        {
                            if let filtered = fi.borrow() && new_sel < filtered.len()
                            {
                                Some( filtered[new_sel] )
                            }
                            else
                            {
                                None
                            }
                        });

                        if let Some( new_idx ) = new_idx
                        {
                            gui.text.set_text( &render_key_values( &entities[new_idx] ) );
                        }
                    }
                }
                else
                {
                    gui.text.set_text( "" );
                    PREV_SEL.with( |p| *p.borrow_mut() = -1 );
                }
                UPDATING_LISTBOX.with( |u| *u.borrow_mut() = false );
            }
        });

        save_entities( gui, &ENTITIES.with( |e| e.borrow().clone() ) ).ok();
    }
}

fn on_save(gui: &EditorWindow)
{
    if let Some( sel ) = gui.list.selection()
    {
        FILTERED_IDXS.with( |f|
        {
            if let filtered = f.borrow() && sel < filtered.len()
            {
                let idx = filtered[sel];
                ENTITIES.with( |ent|
                {
                    if let mut entities = ent.borrow_mut() && idx < entities.len()
                    {
                        entities[idx] = parse_key_values( &gui.text.text() );
                    }
                });
            }
        });
    }

    save_entities( gui, &ENTITIES.with( |ent| ent.borrow().clone() ) ).ok();
    // Exit the editor when saving
    stop_thread_dispatch();
}
// !-TODO-!: Validation needs to be done to check if there were any changes to even save before asking for confirmation.
fn on_close(gui: &EditorWindow, event_data: &EventData)
{
    let choice = modal_message( &gui.window, &MessageParams
    {
        title: "Confirm changes",
        content: &format!( "Save changes to {:?}?", gui.file_path.file_name().unwrap_or_default() ),
        buttons: MessageButtons::YesNoCancel,
        icons: MessageIcons::Question
    });
    
    match choice
    {
        native_windows_gui::MessageChoice::Yes =>
        if let Err( e ) = save_entities( gui, &ENTITIES.with( |ent| ent.borrow().clone() ) )
        {
            eprintln!( "❌ {}", format!( "Failed to save entities: {e}" ).red() );
        }

        native_windows_gui::MessageChoice::No => { },
        _ =>
        {
            if let EventData::OnWindowClose(close_data) = event_data
            {
                close_data.close(false);
            }
            return;
        }
    }
    // Exit the editor when saving
    stop_thread_dispatch();
}

pub fn setup_event_handlers(gui: &'static EditorWindow)
{
    let handle = gui.window.handle;

    bind_event_handler( &handle, &handle, move |evt, event_data, hwnd|
    {
        match evt
        {
            Event::OnButtonClick if hwnd == gui.btn_create.handle => on_create( gui ),
            Event::OnButtonClick if hwnd == gui.btn_clone.handle => on_clone( gui ),
            Event::OnButtonClick if hwnd == gui.btn_delete.handle => on_delete( gui ),
            Event::OnButtonClick if hwnd == gui.btn_save.handle => on_save( gui ),
            Event::OnButtonClick => { }

            Event::OnListBoxSelect if hwnd == gui.list.handle => { on_list_select( gui ); }

            Event::OnTextInput if hwnd == gui.text.handle => { on_text_change( gui ); }
            Event::OnTextInput if hwnd == gui.filter.handle => { apply_filter( gui ); }
            Event::OnTextInput => { }

            Event::OnWindowClose => { on_close( gui, &event_data ); }

            _ => { }
        }
    });
}
