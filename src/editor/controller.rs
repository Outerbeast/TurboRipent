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
use std::path::
{
    Path,
    PathBuf
};

use cursive::
{
    Cursive,
    event::
    {
        Event,
        Key
    },
    view::scroll::Scroller,
    views::
    {
        Dialog,
        EditView,
        LinearLayout,
        NamedView,
        ScrollView,
        SelectView
    }
};

use crate::prelude::*;
use super::view::
{
    self,
    ENTITY_LIST,
    ENTITY_LIST_SCROLL,
    FLAG_TABLE,
    FILTER,
    PROPERTY_TABLE
};
/// Binds view events to controller callbacks
pub(crate) fn with_controller<F>(siv: &mut Cursive, cb: F)
where F: FnOnce(&mut EditorController, &mut Cursive)
{   // The controller is taken out while the callback runs, so re-entrant callbacks cannot double-borrow it
    let Some( mut controller ) = siv.take_user_data::<_>()
    else
    {
        return
    };

    cb( &mut controller, siv );
    siv.set_user_data( controller );
}
/// A snapshot of the editor's mutable state for undo/redo
/// I could do this in a cheaper way, but I'm too lazy to think of one right now
#[derive( Clone )]
struct UndoState
{
    entities: Vec<EntityDictionary>,
    selected_entity: Option<usize>
}
/// State of view/controller
#[derive( Default )]
pub(crate) struct EditorController
{
    entities: Vec<EntityDictionary>,
    saved: Vec<EntityDictionary>,
    filtered_idxs: Vec<usize>,
    selected_entity: Option<usize>,
    rows: Vec<(String, String)>,
    shown_classname: String,
    updating_views: bool,
    undo_stack: Vec<UndoState>,
    redo_stack: Vec<UndoState>,
    file_path: PathBuf
}

impl EditorController
{
    pub(crate) fn new(file_path: &Path, entities: &[EntityDictionary]) -> Self
    {
        Self
        {
            saved: entities.to_vec(),
            entities: entities.to_vec(),
            file_path: file_path.to_path_buf(),
            ..Default::default()
        }
    }
    /// Binds the controller to the TUI and populates the entity list.
    /// This should be called after the root view has been added.
    /// Finally, the event loop must be run afterwards.
    pub(crate) fn register(self, siv: &mut Cursive)
    {
        siv.set_user_data( self );
        siv.add_global_callback( Event::Key( Key::Esc ), |siv| with_controller( siv, EditorController::on_close ) );

        siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<_>|
        {
            list.set_on_select( |siv, pos| with_controller( siv, |ctrl, siv| ctrl.on_list_select( siv, *pos ) ) );
        });

        siv.call_on_name( FILTER, |filter: &mut EditView|
        {
            filter.set_on_edit( |siv, _, _| with_controller( siv, EditorController::apply_filter ) );
        });

        with_controller( siv, EditorController::populate_list );
    }

    fn populate_list(&mut self, siv: &mut Cursive)
    {
        self.rebuild_list( siv, "" );
    }

    fn apply_filter(&mut self, siv: &mut Cursive)
    {
        if self.updating_views
        {
            return;
        }

        self.commit_rows();

        let filter = siv
            .call_on_name( FILTER, |input: &mut EditView| input.get_content().to_string() )
        .unwrap_or_default();

        self.rebuild_list( siv, &filter );
    }

    fn rebuild_list(&mut self, siv: &mut Cursive, filter: &str)
    {
        self.filtered_idxs = self.compute_filtered_idxs( filter );
        self.updating_views = true;

        siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<_>|
        {
            list.clear();

            for (pos, &idx) in self.filtered_idxs.iter().enumerate()
            {
                list.add_item( self.entities[idx].get_classname().to_string(), pos );
            }

            if !self.filtered_idxs.is_empty()
            {
                list.set_selection( 0 );
            }
        });

        self.updating_views = false;
        view::set_entity_count( siv, self.filtered_idxs.len(), self.entities.len(), !filter.trim().is_empty() );

        match self.filtered_idxs.first().copied()
        {
            Some( idx ) => self.select_entity( siv, idx ),
            None =>
            {
                self.selected_entity = None;
                self.rows.clear();
                self.refresh_table( siv );
            }
        }
    }
    /// Recomputes the entity indices that match the given filter
    fn compute_filtered_idxs(&self, filter: &str) -> Vec<usize>
    {
        let filter = filter.trim().to_lowercase();

        self.entities
            .iter()
            .enumerate()
            .filter_map( |(idx, ent)|
            {
                let matches = filter.is_empty()
                    || ent.get_classname().to_lowercase().contains( &filter )
                    || ent.iter().any( |(k, v)| k.to_lowercase().contains( &filter ) 
                    || v.to_lowercase().contains( &filter ) );

                matches.then_some( idx )
            })
        .collect()
    }

    fn on_list_select(&mut self, siv: &mut Cursive, pos: usize)
    {
        if self.updating_views
        {
            return;
        }

        let Some( &idx ) = self.filtered_idxs.get( pos )
        else
        {
            return
        };

        if self.selected_entity == Some( idx )
        {
            return;
        }
        // Commit any pending edits before switching entities; entity switches act as one undo step
        if self.has_pending_row_edits()
        {
            self.push_undo();
        }
        self.commit_rows();
        self.select_entity( siv, idx );
    }
    /// Moves the list highlight and the properties table onto the given entity
    fn set_active_entity(&mut self, siv: &mut Cursive, idx: usize)
    {
        let Some( pos ) = self.filtered_idxs.iter().position( |&i| i == idx )
        else
        {
            return
        };

        self.updating_views = true;
        siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<usize>| list.set_selection( pos ) );
        self.updating_views = false;
        let _ = siv.call_on_name( ENTITY_LIST_SCROLL, |scroll: &mut ScrollView<NamedView<SelectView<usize>>>| scroll.get_scroller_mut().scroll_to_y( pos ) );
        self.select_entity( siv, idx );
    }

    fn select_entity(&mut self, siv: &mut Cursive, idx: usize)
    {
        self.selected_entity = Some( idx );
        self.load_rows();
        self.refresh_table( siv );
    }

    fn load_rows(&mut self)
    {
        self.rows = self
            .selected_entity
            .and_then( |idx| self.entities.get( idx ) )
            .map( |entity| entity.to_kv_pairs() )
        .unwrap_or_default();

        self.shown_classname = self
            .rows
            .iter()
            .find( |row| row.0.trim() == "classname" )
            .map( |row| row.1.clone() )
            .filter( |value| !value.trim().is_empty() )
        .unwrap_or_else( || "<no classname>".to_string() );
    }
    /// Whether the property rows differ from the currently selected entity's stored pairs.
    fn has_pending_row_edits(&self) -> bool
    {
        let Some( idx ) = self.selected_entity
        else
        {
            return false;
        };

        self.rows != self.entities.get( idx ).map( EntityDictionary::to_kv_pairs ).unwrap_or_default()
    }
    /// Writes the property table rows back into the currently selected entity
    fn commit_rows(&mut self)
    {
        let Some( idx ) = self.selected_entity
        else
        {
            return
        };

        if idx < self.entities.len()
        {   // Rebuilds an entity dictionary from the property table rows.
            // Empty keys are dropped, duplicate keys keep their last value.
            let pairs: Vec<_> = self.rows
                .iter()
                .map( |(key, value)| ( key.as_str(), value.as_str() ) )
            .collect();

            self.entities[idx] = EntityDictionary::from_kv_pairs( &pairs );
        }
    }

    fn refresh_table(&mut self, siv: &mut Cursive)
    {
        self.updating_views = true;
        siv.call_on_name( PROPERTY_TABLE, |table: &mut LinearLayout|
        {
            table.clear();

            for (index, row) in self.rows.iter().enumerate()
            {
                table.add_child( view::property_row( index, &row.0, &row.1 ) );
            }

            if self.selected_entity.is_some()
            {
                table.add_child( view::property_add_button() );
            }

        });

        siv.call_on_name( FLAG_TABLE, |table: &mut LinearLayout|
        {
            table.clear();

            if let Some( idx ) = self.selected_entity
            {
                let flags = self.entities[idx].get_spawnflags().unwrap_or( 0 );

                for sf_boxs in view::SPAWNFLAG_BOXES.chunks( 4 )
                {
                    let mut row = LinearLayout::horizontal();

                    for &mask in sf_boxs
                    {
                        row.add_child( view::flag_checkbox( mask, flags & mask != 0 ) );
                    }

                    table.add_child( row );
                }
            }
        });

        self.updating_views = false;

        let classname = self
            .selected_entity
            .and_then( |idx| self.entities.get( idx ) )
            .map( |entity| entity.get_classname() );

        view::set_properties_title( siv, classname );
    }

    fn save_and_rebuild(&mut self, siv: &mut Cursive)
    {
        if let Err( e ) = self.save()
        {
            eprintln!( "❌ Failed to save entities: {e}" );
            return;
        }

        let filter = Self::current_filter( siv );
        self.rebuild_list( siv, &filter );
    }

    fn property_changed(&mut self, siv: &mut Cursive, row: usize, is_key: bool, value: String)
    {
        if self.updating_views || row >= self.rows.len()
        {
            return;
        }

        let was_classname_row = self.rows[row].0.trim() == "classname";

        if is_key
        {
            self.rows[row].0 = value;
        }
        else
        {
            self.rows[row].1 = value;
        }

        if was_classname_row || self.rows[row].0.trim() == "classname"
        {   // Keep the entity listbox entry in sync while the classname is edited
            self.sync_entity_list_label( siv );
        }
    }
    /// Relabels the selected entity's entry in the listbox to match the edited classname.
    /// An empty classname keeps the previously shown label instead of blanking the entry.
    fn sync_entity_list_label(&mut self, siv: &mut Cursive)
    {
        let Some( idx ) = self.selected_entity
        else
        {
            return
        };

        let Some( pos ) = self.filtered_idxs.iter().position( |&i| i == idx )
        else
        {
            return
        };

        let candidate = self
            .rows
            .iter()
            .find( |row| row.0.trim() == "classname" )
            .map( |row| row.1.clone() )
        .unwrap_or_default();

        if !candidate.trim().is_empty()
        {
            self.shown_classname = candidate;
        }

        self.updating_views = true;
        siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<usize>|
        {
            // SelectView has no in-place label mutator, so replace the item at its position;
            // the callback returned by remove_item is intentionally dropped.
            list.remove_item( pos );
            list.insert_item( pos, self.shown_classname.clone(), pos );
            // remove_item nudges the focus up (shown row), so restore it to the original row
            list.set_selection( pos );
        });
        self.updating_views = false;
    }

    pub(crate) fn delete_property_row(&mut self, siv: &mut Cursive, row: usize)
    {
        if row < self.rows.len()
        {
            self.push_undo();
            self.rows.remove( row );// move commit AFTER removing the row
            self.commit_rows();
            self.refresh_table( siv );
        }
    }

    fn save(&mut self) -> anyhow::Result<()>
    {
        EntityDictionary::save_entities( &self.entities, &self.file_path )?;
        self.saved = self.entities.clone();
        
        Ok( () )
    }

    fn current_filter(siv: &mut Cursive) -> String
    {
        siv.call_on_name( FILTER, |input: &mut EditView| input.get_content().to_string() )
            .unwrap_or_default()
    }
    // ============ UNDO / REDO ================
    /// Snapshots the current state onto the undo stack and clears the redo stack.
    fn push_undo(&mut self)
    {
        self.undo_stack.push( UndoState
        {
            entities: self.entities.clone(),
            selected_entity: self.selected_entity
        });
        self.redo_stack.clear();
    }
    /// Restores the given snapshot and rebuilds every derived view/table.
    fn restore(&mut self, siv: &mut Cursive, state: UndoState)
    {
        self.entities = state.entities;
        self.selected_entity = state.selected_entity;
        let filter = Self::current_filter( siv );
        self.filtered_idxs = self.compute_filtered_idxs( &filter );
        self.updating_views = true;

        siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<_>|
        {
            list.clear();

            for (pos, &idx) in self.filtered_idxs.iter().enumerate()
            {
                list.add_item( self.entities[idx].get_classname().to_string(), pos );
            }
        });

        self.updating_views = false;
        view::set_entity_count( siv, self.filtered_idxs.len(), self.entities.len(), !filter.trim().is_empty() );
        // Prefer the restored selection if it is still valid, otherwise fall back to the first visible one
        let effective = self
            .selected_entity
            .filter( |&i| i < self.entities.len() )
            .or( self.filtered_idxs.first().copied() );

        self.selected_entity = effective;

        match effective
        {
            Some( idx ) =>
            {
                if let Some( pos ) = self.filtered_idxs.iter().position( |&i| i == idx )
                {
                    self.updating_views = true;
                    siv.call_on_name( ENTITY_LIST, |list: &mut SelectView<usize>| list.set_selection( pos ) );
                    let _ = siv.call_on_name( ENTITY_LIST_SCROLL, |scroll: &mut ScrollView<NamedView<SelectView<usize>>>| scroll.get_scroller_mut().scroll_to_y( pos ) );
                    self.updating_views = false;
                }

                self.select_entity( siv, idx );
            }

            None =>
            {
                self.rows.clear();
                self.refresh_table( siv );
            }
        }
    }

    pub(crate) fn on_undo(&mut self, siv: &mut Cursive)
    {
        self.commit_rows();
        let Some( state ) = self.undo_stack.pop()
        else
        {
            return
        };
        self.redo_stack.push( UndoState
        {
            entities: self.entities.clone(),
            selected_entity: self.selected_entity
        });
        self.restore( siv, state );
    }

    pub(crate) fn on_redo(&mut self, siv: &mut Cursive)
    {
        let Some( state ) = self.redo_stack.pop()
        else
        {
            return
        };
        self.undo_stack.push( UndoState
        {
            entities: self.entities.clone(),
            selected_entity: self.selected_entity
        });
        self.restore( siv, state );
    }
    // ============ BUTTON CALLBACKS ================
    pub(crate) fn on_create(&mut self, siv: &mut Cursive)
    {
        self.commit_rows();
        self.push_undo();
        self.entities.push( EntityDictionary::new( "new_entity" ) );
        let new_idx = self.entities.len() - 1;
        // Reset the filter so the new entity is visible in the list
        self.updating_views = true;
        siv.call_on_name( FILTER, |input: &mut EditView| input.set_content( "" ) );
        self.updating_views = false;
        self.save_and_rebuild( siv );
        self.set_active_entity( siv, new_idx );
    }

    pub(crate) fn on_clone(&mut self, siv: &mut Cursive)
    {
        self.commit_rows();

        let Some( cloned ) = self.selected_entity.and_then( |idx| self.entities.get( idx ).cloned() )
        else
        {
            return
        };

        self.push_undo();
        self.entities.push( cloned );
        let new_idx = self.entities.len() - 1;
        self.save_and_rebuild( siv );
        self.set_active_entity( siv, new_idx );
    }

    pub(crate) fn on_delete(&mut self, siv: &mut Cursive)
    {
        self.commit_rows();

        let Some( idx ) = self.selected_entity
        else
        {
            return
        };

        self.push_undo();

        if idx < self.entities.len()
        {
            self.entities.remove( idx );
        }

        self.save_and_rebuild( siv );
    }

    pub(crate) fn on_add_property_row(&mut self, siv: &mut Cursive)
    {   // Commit pending edits first so they are not clobbered by the table rebuild
        self.commit_rows();
        self.push_undo();
        self.rows.push( ( String::new(), String::new() ) );
        self.refresh_table( siv );
        // Note sure how to deal with the Result from focus_name, but it's not critical
        match siv.focus_name( &format!( "property_key_{}", self.rows.len() - 1 ) )
        {
            Ok( _o ) => { },
            Err(  _e ) => { },
        }
    }

    pub(crate) fn on_save(&mut self, siv: &mut Cursive)
    {
        self.commit_rows();
        if let Err( e ) = self.save()
        {
            eprintln!( "❌ Failed to save entities: {e}" );
        }
        else
        {
            siv.quit();
        }
    }
    // I don't think this ever gets called.
    pub(crate) fn on_close(&mut self, siv: &mut Cursive)
    {   // No changes to save, just exit
        self.commit_rows();

        if self.entities == self.saved
        {
            siv.quit();
            return;
        }

        let file_name = self.file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let mut confirm = Dialog::text( format!( "Save changes to {file_name}?" ) )
            .title( "Confirm changes" )
            .button( "Yes", |siv|
            {
                with_controller( siv, |ctrl, siv|
                {
                    if let Err( e ) = ctrl.save()
                    {
                        let s_err = format!( "❌ Failed to save entities: {e}" );
                        eprint!( "{s_err}" );
                        super::view::popup( siv, "Error", &s_err, "OK", |_| { } );
                    }
                    
                    siv.quit();
                })
            })
            .button( "No", |siv| siv.quit() )
            .button( "Cancel", |siv| { siv.pop_layer(); } );
        // Relabel to square brackets, matching the other buttons
        for ( label, button ) in [ "[ Yes ]", "[ No ]", "[ Cancel ]" ].into_iter().zip( confirm.buttons_mut() )
        {
            button.set_label_raw( label );
        }

        siv.add_layer( confirm );
    }
}
// ============ CALLBACK GLUE FOR THE VIEW ================
pub(crate) fn property_changed(siv: &mut Cursive, row: usize, is_key: bool, value: String)
{
    with_controller( siv, |ctrl, siv| ctrl.property_changed( siv, row, is_key, value ) );
}

pub(crate) fn flag_changed(siv: &mut Cursive, mask: u32, checked: bool)
{
    with_controller( siv, |ctrl, siv|
    {
        if ctrl.updating_views
        {
            return;
        }

        let Some( idx ) = ctrl.selected_entity
        else
        {
            return
        };

        let mut flags = ctrl.entities.get( idx ).unwrap_or(  &EntityDictionary::new( "new_entity" ) ).get_spawnflags().unwrap_or( 0 );
        ctrl.commit_rows();
        ctrl.push_undo();

        if checked
        {
            flags |= mask;
        }
        else
        {
            flags &= !mask;
        }

        if let Some( entity ) = ctrl.entities.get_mut( idx )
        {
            entity.set_spawnflags( flags );
        }
        // Keep the property-table rows in sync so a later commit/save doesn't drop the change
        let value = 
        if flags == 0 
        { 
            String::new()
        }
        else
        {
            flags.to_string()
        };

        match ctrl.rows.iter_mut().find( |( k, _ )| k == "spawnflags" )
        {
            Some( row ) => row.1 = value,
            None if flags != 0 => ctrl.rows.push( ( "spawnflags".to_string(), value ) ),
            None => { }
        }

        ctrl.refresh_table( siv );
    });
}
