# Universal Editor Patch Notes

## V0.0.1

#### Features
- **IMG2IMG CONVERTER**: Img2Img Converter added (jpeg, png, webp, bmp, tiff, ico file options)
- **TEXT EDITOR**: Header Level 4 on Markdown Mode (E.g ####)
- **TEXT EDITOR**: Adding warning dialogue for leaving an unsaved file.
- **GENERAL**: Adding Logo

#### Debugging
- **TEXT EDITOR**: Codeblock Background now sync's properly (mostly)
- **TEXT EDITOR**: Cursor now no longer desyncs when multi-line codeblocks are present
- **TEXT EDITOR**: Cursor now no longer desyncs on load with old files
- **GENERAL**: Sidebar now no longer shows empty white box instead of down arrow

## V0.0.2

#### Features
- **IMG EDITOR**: First prototype of image editor
- **IMG EDITOR**: Exporting to other image types now allowed (jpeg, png, webp, bmp, tiff, ico)
- **IMG EDITOR**: Option for preservation of metadata
- **IMG EDITOR**: Loading screen for applying filters
- **IMG EDITOR**: Color picker now supports hex codes
- **IMG2IMG CONVERTER**: Error's in image processing now displayed on UI
- **IMG2IMG CONVERTER**: .ico now has auto-scale to 256px option
- **TEXT EDITOR**: Saved Toolbar/File info settings between sessions
- **TEXT EDITOR**: Tooltips for Headers include Keyboard Shortcut
- **TEXT EDITOR**: Automatic view mode detection based off of file extension
- **GENERAL**: Added "System" Theme
- **GENERAL**: Theme setting saves between sessions
- **GENERAL**: Now able to remove files from "recent files" list on sidebar 
- **GENERAL**: Adding trashcan icon instead of "x" for recent files removal

#### Debugging
- **GENERAL**: Removed Visibility of Toolbar/File info settings when not in text editor
- **GENERAL**: Images now show up in recent files when opened from main menu

## V0.0.3

#### Features
- **IMG EDITOR**: Added Keyboard Shortcuts for various tools in image editor
- **IMG EDITOR**: Now allows for image stretching/expanding with resizing.
- **IMG EDITOR**: Corner resizing and selection of text now available
- **IMG EDITOR**: Textbox up/down resizing
- **IMG EDITOR**: Textbox Bold/Italics/Underlining
- **IMG EDITOR**: Textbox Rotating
- **IMG EDITOR**: Roboto and Ubuntu fonts now available within text tool
- **GENERAL**: Top panel now customized to each page
- **GENERAL**: Adding icon to release build executable
- **GENERAL**: Improved Home Page
- **GENERAL**: Can now navigate to home page from any screen or converter
- **GENERAL**: Improved Settings Menu and Patch notes Parsing
- **GENERAL**: Modals now close when clicked off from instead of just using the "x"
  
#### Debugging
- **IMG EDITOR**: Fixed issue only allowing for drawing while dragging
- **IMG EDITOR**: Images now show up in "Recent Files" properly
- **IMG EDITOR**: Text tool now properly shows up in final images
- **IMG EDITOR**: Bold text now shows up in edited canvas, not only in final image
- **IMG EDITOR**: Rotated text now displays properly on export
- **IMG EDITOR**: Rotated text no longer desyncs from canvas position
- **IMG EDITOR**: Can now properly highlight text in text boxes
- **IMG EDITOR**: Selecting another tool now properly deselects active textbox
- **IMG EDITOR**: Sentences now wrap within text boxes
- **IMG EDITOR**: Text boxes now use selected font properly
- **IMG EDITOR**: Cursor no longer gets disconnected from text in text box

## V0.0.4

#### Features
- **IMG EDITOR**: Filters and Transformations moved to top menu bar under "Image" and "Filter"
- **IMG EDITOR**: Eraser now has option to remove background
- **IMG EDITOR**: Crop is now readjustable and displays width and height of crop
- **IMG EDITOR**: Cursor now changes depending on current tool selected
- **IMG EDITOR**: Filter panel is now it's own modal similar to the color picker
- **IMG EDITOR**: Brush presets now available (Regular, Pen, Pencil, Crayon, Marker, Calligraphy Pen, Spray, Watercolor, Charcoal, Airbrush)
- **IMG EDITOR**: Brush shapes now available (Circle, Square, Diamond, Flat (Calligraphy style), Star)
- **IMG EDITOR**: Brush parameters are now changeable
- **IMG EDITOR**: Saving custom brushes as favorites now available
- **IMG EDITOR**: Brush Textures of different types (Rough, Paper, Canvas)
- **TEXT EDITOR**: Now uses Ubuntu and Roboto fonts instead of Mono and Sans
- **TEXT EDITOR**: Default font size and font type can now be selected in settings

#### Debugging
- **IMG EDITOR**: Eraser now properly erases to white background
- **IMG EDITOR**: Cursor now no longer disappears on color picker
- **IMG EDITOR**: Cursor now no longer disappears on unsaved changes screen
- **IMG EDITOR**: Crop tool now properly resizes
- **GENERAL**: Console no longer opens on build executable
- **GENERAL**: Menu version now displays cargo package version
- **GENERAL**: Can no longer click on buttons in the background with the patch notes or settings modal up
