# UniEdit Todo

## General

- [x] Reusable Design
- [x] "System" option to decide light/dark mode
- [x] Create Main Menu
  - [x] Page you go to changes depending on what file you put in on main menu
  - [x] Improved Patch Notes Screen
  - [x] Settings per page
  - [x] Way to go back to main menu
- [x] Sidebar
  - [x] Recent Activity
  - [x] Converter/Screen List
- [x] Unified Top Bar Handling
- [x] Better File System
- [x] Easier way to add new screens/converters (Code only change)
- [x] Add right click context menu to change file name and open file location in "recent files" list

## Screens

- [x] Txt, Md
- [x] Img Processing
- [x] JSON Processing

## Converters

- [x] Img to Img of diff type
- [x] Data Format Converter (.json, .yaml, .toml, .xml, .csv)

## Debugging

### Text Editor Bugs

- [x] Can't Combine Formatting - Medium
- [x] Code block cursor desync
- [x] Code block desync with words right after/before code block
- [x] Can click to non-existent line in markdown mode, desyncing cursor.
- [x] Cannot use strikethrough keyboard shortcut.
- [x] Punctuation after Underlined/Bolding/Italics doesn't render properly
- [x] If there is a asterisk MUCH after the last italicized word, It will think it connected
  - Basically, make sure to not check asterisks that are actively being used in words.
- [x] Bolding is barely noticeable
  - Seems like just turning the markdown text into same color as the plaintext will help a lot.
- [x] Can't put punctuation after subscript or superscript.
- [x] Features in top tool bar are not aligned.
- [x] When using headers, cursor gets disconnected from the actual position
- [x] Change in font size (E.g Bold & Headers) make code block background out of position
- [x] Changing fonts sometimes causes code blocks to desync

### Image Editor Bugs

- [x] Can't just click to color for brush, must drag
- [x] Retouch tool bar is placed randomly, some things very high, some very low
- [x] Color picker
  - [x] Hex code Does not update properly on color picker
  - [x] Button's aren't quite aligned on the color picker
  - [x] Cant see cursor on color picker
- [x] Resize does not work if not aspect ratio locked
- [x] Text tool currently not working
- [x] Eraser is currently just a white brush, messes up when using layers
- [x] Opacity Slider in the layer panel is not that visible and also lags on large images
- [x] Text Corner Resizing delta too fast, hard to select
- [x] Eraser doesn't erase to white background, completely removes background
- [x] Crop suddenly jumps when resizing vertically or horizontally
- [x] Can't see cursor on unsaved changes popup
- [x] Weird sharpen artifacting on high amount in retouch
- [x] Zoom, aspect ratio, and color cover potions of retouch tool on small monitors
- [x] Preview button should be able to be turned off and on, rather than having to click cancel
- [x] Some text on buttons is white in light mode
- [x] Hover effects of buttons only work in light mode
- [x] Some buttons within the brush settings in light mode are colored white with white background
- [x] Sliders are only visible in dark mode
- [x] Can't scroll down on the color picker
- [x] Text is slightly smaller when stamped then when actually edited
- [x] Text get's cut off if textbox isn't big enough
- [x] Selecting text should auto move to text layer
- [x] Un-selecting on a text box with no text should auto remove the text box
- [x] Text Issues
  - [x] Bold only shows up when fully saved
  - [x] Rotated text has spots in final image
  - [x] Rotated text does not show up in the same place when saved
  - [x] Cannot highlight text within text box
  - [x] When selecting another tool, text box should be deselected
  - [x] Text doesn't rotate with canvas on transform
  - [x] Sentences don't wrap properly within text boxes
  - [x] Doesn't actually use font's listed in text box
  - [x] Highlighting shows up with Ctrl+A, but doesn't actually select properly.
  - [x] Cursor getting disconnected from where text is actually being written.
  - [x] Exported Image doesn't fully contain wrapped text if there are no spaces (E.g one really long word that spans multiple lines)
  - [x] Vertical and Horizontal Rotations do not rotate textbox properly

### Json Editor Bugs

- [x] Going into Tree view causes "modified" tag to show up
- [x] Error popup in text view should hug right side, also text leaves the size of the box
- [x] File information and search bar backgrounds do not go all the way to the right of the screen
- [x] Format and sort combobox not centered in the toolbar
- [x] Search not navigating to results properly
- [x] Does not allow undo for any text changes
- [x] Json Editor crashes when trying to navigate back at least 2 parents
- [x] Text view very laggy with large JSON files
- [x] Scrolling to the bottom of a large JSON file in text view with a small screen sometimes causes "scroll bounce"
  - Doesn't allow user to scroll all the way to the bottom of the file
- [x] Does not use the raw data of the JSON file for text view, uses edited version
- [x] Long numbers turn into scientific value, and save as scientific value instead of staying as the long number
- [x] Can't ctrl+s save
- [x] Saving does not update "modified" file information value

### Other Bugs

- [x] Sidebar down arrow shows as empty square
- [x] Top bar "View" should not show text editor options when not on text editor
- [x] Can click on buttons in the background of the main menu with patch notes/settings modal up

## Features

### Text Editor

- [x] Keyboard Shortcuts
- [x] Markdown Loader
- [x] Download/Save Files
- [x] Bold, Italics, Underline, Strikethrough
  - [x] Make Bold stand out more
- [x] Font, Font Size
- [x] Headers
  - [x] Button to add headers
- [x] Superscript, Subscript
- [x] Clickable Links
- [x] Hover features in toolbar to know what they are (E.g hover B to show "bold")
- [x] Show what file is being edited
- [x] Turn off Toolbar
- [x] Make Code blocks more readable.
  - [x] Add titles to code blocks (E.g "bash" or "rust")
  - [x] Add newlines after and before code in code block.
- [x] Showing saved/unsaved
- [x] Opening .md file automatically sets markdown mode
- [x] Markdown checkbox lists (for example this one)
- [x] Use given fonts instead of egui font families (E.g ubuntu and roboto)
- [x] Default font and font size support
- [x] Add way to go to file location
- [x] Add way to rename file
- [x] Add way to convert file from txt to md and vice versa
- [x] Ability to load tables in markdown mode

### Image Editor

- [x] Layer system
  - [x] Add
  - [x] Reorder
  - [x] Merge
  - [x] Opacity
- [x] Retouch tool
  - [x] Blur
  - [x] Sharpen
  - [x] Smudge
  - [x] Vibrance
  - [x] Saturation
  - [x] Temperature
  - [x] Brightness
  - [x] Pixelate
  - [x] Clearer slider for dark/light amount on brightness
  - [x] Clearer slider for cold/warm amount on temperature
  - [x] Clearer slider for difference between vibrant and non vibrant for vibrance
  - [x] Max sharpen amount?
- [x] Color picker
  - [x] Recent colors option in color picker
  - [x] Bigger/Clearer Color picker
  - [x] Color code to color option
  - [x] Add Favorites to color picker (with keyboard shortcuts)
- [x] Better text selection/writing tool
  - [x] Easy selection of text
  - [x] Resize by corner drag
  - [x] Resize Up/Down
  - [x] Bold/Italics/Underlined Text
  - [x] Font picker
  - [x] Rotate
- [x] Brush Settings Panel
  - [x] Writing tools
    - [x] Pen
    - [x] Pencil
    - [x] Crayon
  - [x] Custom brush shapes
- [x] Keyboard Shortcuts
- [x] Canvas size (extend without scaling)
- [x] Export with metadata
- [x] Export to other image types
- [x] Filter preview before apply (on filter panel)
- [x] Turn filter panel into it's own modal
- [x] Improve button look when the screen/resolution is smaller
- [x] Images show up in recent files
- [x] Make image converter and image editor use the same export function
- [x] Add different cursors for different functions/tools
- [x] Crop should show preview of the size of the image
- [x] Center hue picker and color square within color picker

### Json Editor

- [x] Undo and Redo can be moved to top bar
- [x] Give the ability to show/hide file information
- [x] Create JSON styling file, move out of ui file
- [x] Add line numbers to text view

### CONVERTER: Img 2 Img

- [x] Display Error (For example, width > 256 for ico conversion)

## Other

- [x] Create Todo List
- [x] Clear/Remove recent files
  - [x] Trashcan icon instead of x
