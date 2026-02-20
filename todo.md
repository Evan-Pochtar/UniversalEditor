# UniEdit Todo

## General

- [x] Reusable Design
- [x] "System" option to decide light/dark mode
- [ ] Create Main Menu
  - [x] Page you go to changes depending on what file you put in on main menu
  - [x] Improved Patch Notes Screen
  - [ ] Add About Page
  - [x] Settings per page
  - [x] Way to go back to main menu
- [x] Sidebar
  - [x] Recent Activity
  - [x] Converter/Screen List
  - [ ] Change checkmark close/open sidebar to a small arrow.
- [ ] Keyboard shortcuts to close/open sidebar, toolbar, etc.
- [x] Unified Top Bar Handling
- [x] Better File System

## Screens

- [x] Txt, Md
- [ ] Word Docs, ect.
- [x] Img Processing
- [ ] Video Processing
- [ ] JSON Processing
- [ ] CSV, Excel, ect.
- [ ] PDF
- [ ] Zip/Unzip
- [ ] Code Editor?

## Converters

- [ ] Video to Gif, Gif to Video
- [ ] Video Compressor
- [x] Img to Img of diff type
- [ ] Video to Audio File

## Debugging

### Text Editor Bugs

- [ ] Cannot highlight and scroll at the same time? - Hard
- [ ] Can't Combine Formatting - Medium
- [ ] At 20 Font size with Sans font, code block doesn't format correctly.
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

### Image Editor Bugs

- [x] Can't just click to color for brush, must drag
- [x] Color picker
  - [x] Hex code Does not update properly on color picker
  - [x] Button's aren't quite aligned on the color picker
  - [x] Cant see cursor on color picker
- [x] Resize does not work if not aspect ratio locked
- [x] Text tool currently not working
- [x] Text Corner Resizing delta too fast, hard to select
- [x] Eraser doesn't erase to white background, completely removes background
- [x] Crop suddenly jumps when resizing vertically or horizontally
- [x] Can't see cursor on unsaved changes popup
- [ ] Text Issues
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
  - [ ] Exported Image doesn't fully contain wrapped text if there are no spaces (E.g one really long word that spans multiple lines)
  - [ ] Vertical and Horizontal Rotations do not rotate textbox properly

### Other Bugs

- [x] Sidebar down arrow shows as empty square
- [x] Top bar "View" should not show text editor options when not on text editor
- [ ] Save warning dialogue does not show up on application close
- [ ] Can click on buttons in the background of the main menu with patch notes/settings modal up

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
- [ ] Latex?

### Image Editor

- [ ] Layer system
  - [ ] Add
  - [ ] Reorder
  - [ ] Merge
  - [ ] Opacity
- [ ] Selection tools
  - [ ] Ellipse
  - [ ] Free-hand lasso
  - [ ] Magic wand
  - [ ] Copy/paste/cut within selection
- [ ] Gradient tool
  - [ ] Linear
  - [ ] Radial
- [ ] Cutout tool
- [ ] Filter tool
  - [ ] Blur
  - [ ] Sharpen
  - [ ] Smudge
  - [ ] Vibrance
  - [ ] Saturation
  - [ ] Temperature
  - [ ] Brightness
- [ ] Writing tools
  - [ ] Pen
  - [ ] Pencil
  - [ ] Crayon
- [ ] Shape tool
- [ ] Performance
  - [ ] Brush is slightly laggy, make more smooth on large images
  - [x] Increase performance of filters on bigger images (E.g Blur)
  - [ ] General performance and usability improvements
  - [x] Add loading screen for filters etc.
- [x] Color picker
  - [x] Recent colors option in color picker
  - [x] Bigger/Clearer Color picker
  - [x] Color code to color option
  - [ ] Add opacity to color picker
- [x] Better text selection/writing tool
  - [x] Easy selection of text
  - [x] Resize by corner drag
  - [x] Resize Up/Down
  - [x] Bold/Italics/Underlined Text
  - [x] Font picker
  - [x] Rotate
- [ ] More brush presets or options
  - [ ] Custom brush shapes
- [x] Keyboard Shortcuts
- [ ] Import Images into Canvas
- [ ] Pattern/texture fill
- [ ] Perspective/affine warp
- [ ] Edge detection filter
- [x] Canvas size (extend without scaling)
- [x] Export with metadata
- [x] Export to other image types
- [ ] Snap to grid for text/crop
- [ ] Filter preview before apply
- [ ] Drag-and-drop image open
- [x] Improve button look when the screen/resolution is smaller
- [x] Images show up in recent files
- [x] Make image converter and image editor use the same export function
- [x] Add different cursors for different functions/tools
- [x] Crop should show preview of the size of the image

### CONVERTER: Img 2 Img

- [ ] Keyboard Shortcuts
- [x] Display Error (For example, width > 256 for ico conversion)

## Other

- [x] Create Todo List
- [x] Clear/Remove recent files
  - [x] Trashcan icon instead of x

## Testing

- [ ] Create Testing Framework
