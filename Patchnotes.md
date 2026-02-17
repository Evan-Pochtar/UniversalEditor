# Universal Editor Patchnotes

## V0.0.1

#### Features
- **IMG2IMG COVNERTER**: Img2Img Converter added (jpeg, png, webp, bmp, tiff, ico file options)
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
- **IMG EDITOR**: Color picker now supports hexcodes
- **IMG2IMG CONVERTER**: Error's in image processing now dispalyed on UI
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
- **GENERAL**: Improved Settings Menu and Patchnotes Parsing
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
- **IMG EDITOR**: Cusor no longer gets disconnected from text in text box
