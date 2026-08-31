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
    io,
    error::Error,
    path::Path
};

use crossterm::terminal;
use cursive::
{
    Cursive,
    theme::
    {
        Color,
        ColorStyle,
        PaletteColor,
        PaletteStyle,
        Theme,
    },
    utils::markup::StyledString,
    view::
    {
        Nameable,
        Resizable,
        Scrollable,
        View
    },
    views::
    {
        Button,
        Checkbox,
        Dialog,
        DummyView,
        EditView,
        LinearLayout,
        NamedView,
        Panel,
        ScrollView,
        SelectView,
        TextView,
        ThemedView
    }
};

use crate::prelude::*;

use super::controller::
{
    EditorController,
    with_controller,
    property_changed,
    flag_changed
};

pub(crate) const ENTITY_LIST: &str = "entity_list";
pub(crate) const ENTITY_LIST_SCROLL: &str = "entity_list_scroll";
const ENTITY_LIST_PANEL: &str = "entity_list_panel";
pub(crate) const FILTER: &str = "entity_filter";
pub(crate) const PROPERTY_TABLE: &str = "property_table";
pub(crate) const FLAG_TABLE: &str = "flag_table";
const PROPERTIES_PANEL: &str = "properties_panel";
pub(crate) const SPAWNFLAG_BOXES: [u32; 32] =
{
    let mut masks = [0; 32];
    let mut i = 0;

    while i < 32
    {
        masks[i] = 1 << i;
        i += 1;
    }

    masks
};

const LIST_WIDTH: usize = 36;
const KEY_WIDTH: usize = 28;
// Theme colours
const BACKGROUND_COLOUR: Color = Color::TerminalDefault;
const BORDER_COLOUR: Color = Color::Rgb( 60, 150, 220 ); // Panel/dialog borders + regular text share this colour
const TEXT_COLOUR: Color = Color::Rgb( 128, 128, 128 ); // Entity list text
const TITLE_COLOUR: Color = Color::Rgb( 230, 110, 50 ); // Panel/dialog titles
const HIGHLIGHT_COLOUR: Color = Color::Rgb( 30, 30, 30 );
const HIGHLIGHT_TEXT_COLOUR: Color = Color::Rgb( 128, 255, 128 );// Colour of the entity thats selected in the list
const BUTTON_COLOUR: Color = Color::Rgb( 128, 255, 128 );
const CURSOR_FRONT_COLOUR: Color = Color::Rgb( 255, 255, 255 );
const CURSOR_BACK_COLOUR: Color = Color::Rgb( 0, 0, 255 );
/// Must mirror the wrap chain in build(): NamedView > ScrollView > NamedView, wrapped in ThemedView
type EntityListPanel = Panel<ThemedView<NamedView<ScrollView<NamedView<SelectView<usize>>>>>>;// Ogres have layers
type PropertyTablePanel = Panel<LinearLayout>;
/// Creates a button rendered with square brackets instead of cursive's angle brackets
fn button<F>(label: &str, cb: F) -> Button
where F: Fn(&mut Cursive) + Send + Sync + 'static
{
    Button::new_raw( StyledString::styled( label, BUTTON_COLOUR ), cb )
}
/// Creates a popup dialog with a single button. Doesn't block.
pub(crate) fn popup<F>(siv: &mut Cursive, title: &str, message: &str, button_text: &str, cb: F)
where F: Fn(&mut Cursive) + Send + Sync + 'static,
{
    siv
        .add_layer( Dialog::text( message )
        .title( title )
        .button( button_text, cb ) );
}
/// Applies a black-on-default theme to the root view
fn apply_theme() -> Theme
{
    let mut theme = Theme::terminal_default();
    theme.palette[PaletteColor::Background] = BACKGROUND_COLOUR;
    theme.palette[PaletteColor::View] = BACKGROUND_COLOUR;
    theme.palette[PaletteColor::Primary] = BORDER_COLOUR;
    theme.palette[PaletteColor::Secondary] = Color::Rgb( 196, 196, 196 );
    theme.palette[PaletteColor::Highlight] = HIGHLIGHT_COLOUR;
    theme.palette[PaletteColor::HighlightText] = HIGHLIGHT_TEXT_COLOUR;
    theme.palette[PaletteColor::TitlePrimary] = TITLE_COLOUR;
    theme.palette[PaletteColor::TitleSecondary] = TITLE_COLOUR;
    theme.palette[PaletteStyle::EditableTextCursor] = ColorStyle::new( CURSOR_FRONT_COLOUR, CURSOR_BACK_COLOUR ).into();
    theme.palette[PaletteStyle::HighlightInactive] = ColorStyle::new( HIGHLIGHT_TEXT_COLOUR, HIGHLIGHT_COLOUR ).into();

    theme
}
/// Builds view, controller then runs the editor
pub(crate) fn display(file_path: &Path, entities: &[EntityDictionary]) -> Result<(), Box<dyn Error>>
{
    let title = format!( "{APPNAME} - {}", file_path.file_name().unwrap_or_default().to_string_lossy() );
    crossterm::execute!( io::stdout(), terminal::SetTitle( &title ) )?;
    let mut siv = cursive::default();
    siv.set_theme( apply_theme() );
    siv.add_fullscreen_layer( build() );
    EditorController::new( file_path, entities ).register( &mut siv );

    siv.try_run()
}

fn build() -> impl View
{   // Entity classname ListBox on the left
    let mut list_theme = apply_theme();
    list_theme.palette[PaletteColor::Primary] = TEXT_COLOUR;

    let list = Panel::new(
    ThemedView::new( list_theme, SelectView::<usize>::new()
        .with_name( ENTITY_LIST )
        .scrollable()
        .with_name( ENTITY_LIST_SCROLL ) ) )
    .title( "Entities" )
    .with_name( ENTITY_LIST_PANEL );

    let filter = LinearLayout::vertical()
        .child( TextView::new( "Search 🔍" ) )
        .child( EditView::new()
            .filler( " " )
            .with_name( FILTER ).full_width() )
        .full_width();
    // Entity Properties table to the right of listbox
    let properties_header = LinearLayout::horizontal()
        .child( TextView::new( "Key" ).fixed_width( KEY_WIDTH ) )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( TextView::new( "Value" ) );

    let properties_table = Panel::new( LinearLayout::vertical()
        .child( properties_header )
        .child( ScrollView::new( LinearLayout::vertical().with_name( PROPERTY_TABLE ) )
        .scroll_x( false ) ) )
        .title( "Properties" )
        .with_name( PROPERTIES_PANEL )
    .full_height();

    let flags_panel = Panel::new( ScrollView::new( LinearLayout::vertical().with_name( FLAG_TABLE ) ) )
        .title( "Flags" )
        .fixed_height( 11 );
    // Buttons row
    let btn_create = button( "🆕Create", |siv| with_controller( siv, EditorController::on_create ) );
    let btn_clone = button( "🖨️Clone", |siv | with_controller( siv, EditorController::on_clone ) );
    let btn_delete = button( "🗑️Delete", |siv| with_controller( siv, EditorController::on_delete ) );
    let btn_undo = button( "↩️Undo", |siv| with_controller( siv, EditorController::on_undo ) );
    let btn_redo = button( "↪️Redo", |siv| with_controller( siv, EditorController::on_redo ) );
    let btn_save = button( "💾Save", |siv| with_controller( siv, EditorController::on_save ) );

    let button_row = LinearLayout::horizontal()
        .child( btn_create )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( btn_clone )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( btn_delete )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( btn_undo )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( btn_redo )
        .child( DummyView{ }.full_width() )
        .child( btn_save );

    let buttons = LinearLayout::vertical()
        .child( DummyView{ }.full_width() )
        .child( button_row )
        .fixed_height( 2 );

    let right = LinearLayout::vertical()
        .child( properties_table )
        .child( flags_panel )
        .child( buttons );

    let layout = LinearLayout::horizontal()
        .child( LinearLayout::vertical()
        .child( list.full_height() )
        .child( filter )
            .fixed_width( LIST_WIDTH ) )
        .child( DummyView{ }.fixed_width( 1 ) )
        .child( right.full_width() );

    layout.full_screen()
}
/// Updates the entity list panel title with the shown/total entity count
pub(crate) fn set_entity_count(siv: &mut Cursive, shown: usize, total: usize, filtered: bool)
{
    let title = 
    if filtered
    {
        format!( "Entities ({shown}/{total})" )
    }
    else
    {
        format!( "Entities ({total})" )
    };

    let updated = siv.call_on_name( ENTITY_LIST_PANEL, |panel: &mut EntityListPanel| panel.set_title( title ) );
    debug_assert!( updated.is_some(), "EntityListPanel alias out of sync with build() in view.rs" );
}
/// Updates the properties panel title to show the selected entity classname
pub(crate) fn set_properties_title(siv: &mut Cursive, classname: Option<&str>)
{
    let updated = siv.call_on_name( PROPERTIES_PANEL, |panel: &mut PropertyTablePanel|
    {
        panel.set_title(
        match classname
        {
            Some( classname ) => format!( "Properties: {classname}" ),
            None => "Properties".to_string()
        });
    });
    debug_assert!( updated.is_some(), "PropertyTablePanel alias out of sync with build() in view.rs" );
}
/// Builds one editable Key/Value table row for the given entity property
pub(crate) fn property_row(index: usize, key: &str, value: &str) -> LinearLayout
{
    LinearLayout::horizontal()
        .child( EditView::new()
            .content( key )
            .filler( " " )
            .on_edit( move |siv, content, _| property_changed( siv, index, true, content.to_string() ) )
        .with_name( format!( "property_key_{index}" ) )
        .fixed_width( KEY_WIDTH ) )
        .child( DummyView{}.fixed_width( 1 ) )
        .child( EditView::new()
            .content( value )
            .filler( " " )
            .on_edit( move |siv, content, _| property_changed( siv, index, false, content.to_string() ) )
        .with_name( format!( "property_value_{index}" ) )
        .full_width() )
        .child( Button::new_raw( StyledString::styled( "❌", BUTTON_COLOUR ), 
            move |siv| with_controller( siv, |ctrl, siv| ctrl.delete_property_row( siv, index ) ) ) )
}
/// Creates a right-aligned ➕ button appended below the last key/value row of the properties table
pub(crate) fn property_add_button() -> LinearLayout
{
    LinearLayout::horizontal()
    .child( DummyView{ }.full_width() )
    .child( button( "➕", |siv|
    {
        with_controller( siv, |ctrl, siv| ctrl.on_add_property_row( siv ) );
    }))
}
/// Creates one spawnflag checkbox for the properties panel
pub(crate) fn flag_checkbox(mask: u32, checked: bool) -> impl View
{
    let mut theme = Theme::default();
    theme.palette[PaletteColor::View] = BACKGROUND_COLOUR;
    theme.palette[PaletteColor::Primary] = BUTTON_COLOUR;
    theme.palette[PaletteColor::Secondary] = BUTTON_COLOUR;
    theme.palette[PaletteColor::Highlight] = BUTTON_COLOUR;

    LinearLayout::horizontal()
    .child( ThemedView::new( theme, Checkbox::new()
        .with_checked( checked )
        .on_change( move |siv, checked| flag_changed( siv, mask, checked ) ) ) )
    .child( DummyView{}.fixed_width( 1 ) )
    .child( TextView::new( mask.to_string() ) )
        .fixed_width( 20 )
}
