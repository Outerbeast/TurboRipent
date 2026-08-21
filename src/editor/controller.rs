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
use std::cell::RefCell;

use crossterm::style::Stylize;
use native_windows_gui::
{
    Event,
    EventData,
    MessageButtons,
    MessageIcons,
    MessageParams,
    bind_event_handler,
    dispatch_thread_events,
    modal_message,
    stop_thread_dispatch
};

use super::view::EditorWindow;
use crate::prelude::*;

thread_local!
{
    static CTRL: RefCell<Option<EditorController>> = const { RefCell::new( None ) };
}

macro_rules! with_controller
{
    ( $gui:expr, |$ctrl:ident, $app:ident| $body:expr ) =>
    {
        CTRL.with( |c|
        {
            if let Some( ref mut $ctrl ) = *c.borrow_mut()
            {
                let $app = $gui;
                $body
            }
        })
    };
}

pub(crate) struct EditorController
{
    entities: Vec<EntityDictionary>,
    saved: Vec<EntityDictionary>,
    filtered_idxs: Vec<usize>,
    prev_sel: i32,
    updating_listbox: bool
}

impl EditorController
{
    pub fn new(gui: &'static EditorWindow, entities: Vec<EntityDictionary>) -> Self
    {
        let handle = gui.window.handle;

        bind_event_handler( &handle, &handle, move |evt, event_data, hwnd|
        {
            match evt
            {
                Event::OnButtonClick if hwnd == gui.btn_create.handle => with_controller!( gui, |ctrl, app| ctrl.on_create( app ) ),
                Event::OnButtonClick if hwnd == gui.btn_clone.handle => with_controller!( gui, |ctrl, app| ctrl.on_clone( app ) ),
                Event::OnButtonClick if hwnd == gui.btn_delete.handle => with_controller!( gui, |ctrl, app| ctrl.on_delete( app ) ),
                Event::OnButtonClick if hwnd == gui.btn_save.handle => with_controller!( gui, |ctrl, app| ctrl.on_save( app ) ),
                Event::OnButtonClick => { }
                Event::OnListBoxSelect if hwnd == gui.list.handle => with_controller!( gui, |ctrl, app| ctrl.on_list_select( app ) ),
                Event::OnTextInput if hwnd == gui.text.handle => with_controller!( gui, |ctrl, app| ctrl.on_text_change( app ) ),
                Event::OnTextInput if hwnd == gui.filter.handle => with_controller!( gui, |ctrl, app| ctrl.apply_filter( app ) ),
                Event::OnTextInput => { }
                Event::OnWindowClose => with_controller!( gui, |ctrl, app| ctrl.on_close( app, &event_data ) ),
                _ => { }
            }
        });

        Self
        {
            saved: entities.clone(),
            entities,
            filtered_idxs: vec![],
            prev_sel: -1,
            updating_listbox: false
        }
    }
    /// Binds the controller to the GUI and populates the listbox with the entity names.
    /// This should be called after the GUI has been created.
    /// Finally, it starts the event loop.
    pub fn register(self, gui: &'static EditorWindow)
    {
        CTRL.with( |c| *c.borrow_mut() = Some( self ) );
        with_controller!( gui, |ctrl, app| ctrl.populate_listbox( app ) );
        dispatch_thread_events();
    }

    fn save(&mut self, gui: &EditorWindow)
    {
        if let Err( e ) = EntityDictionary::save_entities( &self.entities, &gui.file_path )
        {
            let title = "Error";
            let content = format!( "Failed to save entities: {e}" );

            let _ = modal_message( &gui.window, &MessageParams
            { 
                title,
                content: &content,
                buttons: MessageButtons::Ok,
                icons: MessageIcons::Error
            });

            eprintln!( "❌ {}", content.red() )
        }
        else
        {
            self.saved = self.entities.clone()
        }
    }

    fn populate_listbox(&mut self, gui: &EditorWindow)
    {
        self.updating_listbox = true;
        gui.list.clear();
        self.filtered_idxs.clear();

        for (i, name) in self.entities.iter().map( |e| e.get_classname() ).enumerate()
        {
            gui.list.push( name.to_string() );
            self.filtered_idxs.push( i );
        }

        self.updating_listbox = false;

        if !self.filtered_idxs.is_empty()
        {
            gui.list.set_selection( Some( 0 ) );

            if let Some( sel ) = gui.list.selection()
            && let Some( &idx ) = self.filtered_idxs.get( sel )
            && let Some( entity ) = self.entities.get( idx )
            {
                gui.text.set_text( &entity.render_keyvalues().unwrap_or_default() );
            }
                
            self.prev_sel = 0;
        }
    }

    fn apply_filter(&mut self, gui: &EditorWindow)
    {
        let filter = gui.filter.text().trim().to_lowercase();
        self.updating_listbox = true;
        gui.list.clear();
        self.filtered_idxs.clear();

        if filter.is_empty()
        {
            for (i, name) in self.entities.iter().map( |e| e.get_classname() ).enumerate()
            {
                gui.list.push( name.to_string() );
                self.filtered_idxs.push( i );
            }
        }
        else
        {
            for (i, ent) in self.entities.iter().enumerate()
            {
                for (k, v) in ent.iter()
                {
                    if k.to_lowercase().contains( &filter ) || v.to_lowercase().contains( &filter )
                    {
                        gui.list.push( ent.get_classname().to_string() );
                        self.filtered_idxs.push( i );

                        break;
                    }
                }
            }
        }

        self.updating_listbox = false;

        if !self.filtered_idxs.is_empty()
        {
            gui.list.set_selection( Some( 0 ) );

            if let Some( sel ) = gui.list.selection()
            && let Some( &idx ) = self.filtered_idxs.get( sel )
            && let Some( entity ) = self.entities.get( idx )
            {
                gui.text.set_text( &entity.render_keyvalues().unwrap_or_default() );
            }

            self.prev_sel = 0;
        }
        else
        {
            gui.list.set_selection( None );
            gui.text.set_text( "" );
            self.prev_sel = -1;
        }
    }

    fn on_list_select(&mut self, gui: &EditorWindow)
    {
        if self.updating_listbox
        {
            return;
        }

        let Some( sel ) = gui.list.selection() else { return };
        let Some( &idx ) = self.filtered_idxs.get( sel ) else { return };

        if self.prev_sel >= 0
        {
            let prev = self.prev_sel as usize;

            if prev < self.entities.len()
            {
                self.entities[prev] = EntityDictionary::parse_keyvalues( &gui.text.text() );
            }
        }

        if let Some( entity ) = self.entities.get( idx )
        {
            gui.text.set_text( &entity.render_keyvalues().unwrap_or_default() );
        }

        self.prev_sel = idx as i32;
    }

    fn on_text_change(&mut self, gui: &EditorWindow)
    {
        if self.updating_listbox
        {
            return;
        }

        let Some( sel ) = gui.list.selection()
        else
        {
            return
        };

        let Some( &idx ) = self.filtered_idxs.get( sel ) 
        else
        {
            return
        };

        if idx < self.entities.len()
        {
            self.entities[idx] = EntityDictionary::parse_keyvalues( &gui.text.text() );
        }

        self.refresh_listbox_item( gui, sel as i32 );
    }

    fn refresh_listbox_item(&mut self, gui: &EditorWindow, sel: i32)
    {
        self.updating_listbox = true;

        if sel >= 0 && (sel as usize) < self.filtered_idxs.len()
        {
            let idx = self.filtered_idxs[sel as usize];

            if let Some( entity ) = self.entities.get( idx )
            {
                let new_collection =
                {
                    let collection = gui.list.collection();

                    if (sel as usize) < collection.len()
                    {
                        let mut new_collection = collection.clone();
                        new_collection[sel as usize] = entity.get_classname().to_string();
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
        }

        self.updating_listbox = false;
    }
    // ============ CALLBACKS ================
    fn on_create(&mut self, gui: &EditorWindow)
    {
        let new_entity = EntityDictionary::new( "new_entity" );
        self.entities.push( new_entity );
        let idx = self.entities.len() - 1;

        self.updating_listbox = true;
        gui.list.push( self.entities[idx].get_classname().to_string() );
        self.filtered_idxs.push( idx );
        gui.list.set_selection( Some( gui.list.len() - 1 ) );
        self.updating_listbox = false;
        self.prev_sel = idx as i32;
        gui.text.set_text( &self.entities[idx].render_keyvalues().unwrap_or_default() );

        self.save( gui );
    }

    fn on_clone(&mut self, gui: &EditorWindow)
    {
        let Some( sel ) = gui.list.selection()
        else
        {
            return
        };

        let Some( &idx ) = self.filtered_idxs.get( sel )
        else
        {
            return
        };

        if idx >= self.entities.len()
        {
            return;
        }

        let cloned = self.entities[idx].clone();
        self.entities.push( cloned );
        let new_idx = self.entities.len() - 1;

        self.updating_listbox = true;
        gui.list.push( self.entities[new_idx].get_classname().to_string() );
        self.filtered_idxs.push( new_idx );
        gui.list.set_selection( Some( gui.list.len() - 1 ) );
        self.updating_listbox = false;
        self.prev_sel = new_idx as i32;
        gui.text.set_text( &self.entities[new_idx].render_keyvalues().unwrap_or_default() );

        self.save( gui );
    }

    fn on_delete(&mut self, gui: &EditorWindow)
    {
        let Some( sel ) = gui.list.selection()
        else
        {
            return
        };

        let Some( &idx ) = self.filtered_idxs.get( sel )
        else
        {
            return
        };

        if idx >= self.entities.len()
        {
            return;
        }

        self.entities.remove( idx );
        self.updating_listbox = true;
        gui.list.remove( sel );
        self.filtered_idxs.remove( sel );
        let new_len = gui.list.len();

        if sel < new_len
        {
            gui.list.set_selection( Some( sel ) );

            if let Some( new_sel ) = gui.list.selection()
            && let Some( &new_idx ) = self.filtered_idxs.get( new_sel )
            && let Some( entity ) = self.entities.get( new_idx )
            {
                gui.text.set_text( &entity.render_keyvalues().unwrap_or_default() );
            }
        }
        else if new_len > 0
        {
            gui.list.set_selection( Some( new_len - 1 ) );

            if let Some( new_sel ) = gui.list.selection()
            && let Some( &new_idx ) = self.filtered_idxs.get( new_sel )
            && let Some( entity ) = self.entities.get( new_idx )
            {
                gui.text.set_text( &entity.render_keyvalues().unwrap_or_default() );
            }
        }
        else
        {
            gui.text.set_text( "" );
            self.prev_sel = -1;
        }

        self.updating_listbox = false;
        self.save( gui );
    }

    fn on_save(&mut self, gui: &EditorWindow)
    {
        if let Some( sel ) = gui.list.selection()
        && let Some( &idx ) = self.filtered_idxs.get( sel )
        && idx < self.entities.len()
        {
            self.entities[idx] = EntityDictionary::parse_keyvalues( &gui.text.text() );
        }

        self.save( gui );
        stop_thread_dispatch();
    }

    fn on_close(&mut self, gui: &EditorWindow, event_data: &EventData)
    {   // No changes to save, just exit
        if self.entities == self.saved
        {
            stop_thread_dispatch();
            return;
        }

        let choice = modal_message( &gui.window, &MessageParams
        {
            title: "Confirm changes",
            content: &format!( "Save changes to {:?}?", gui.file_path.file_name().unwrap_or_default() ),
            buttons: MessageButtons::YesNoCancel,
            icons: MessageIcons::Question
        });

        match choice
        {
            native_windows_gui::MessageChoice::Yes => self.save( gui ),
            native_windows_gui::MessageChoice::No => { }
            _ =>
            {   // Cancelled or closed with the X button, so go back
                if let EventData::OnWindowClose( close_data ) = event_data
                {
                    close_data.close( false );
                }

                return;
            }
        }

        stop_thread_dispatch();
    }
}

