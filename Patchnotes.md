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
- **IMG EDITOR**: Favorites added to the color picker (with keyboard shortcuts)
- **IMG EDITOR**: Retouch tool added (Blur, Sharpen, Smudge, Vibrance, Saturation, Temperature, and Brightness)
- **IMG EDITOR**: Added pixelate to retouch tool
- **IMG EDITOR**: Added Gradient sliders for Hue, Brightness, Temperature, and Vibrance on both Filter Panel and Retouch tool
- **IMG EDITOR**: Added Filter panel preview
- **TEXT EDITOR**: Now uses Ubuntu and Roboto fonts instead of Mono and Sans
- **TEXT EDITOR**: Default font size and font type can now be selected in settings
- **TEXT EDITOR**: Adding functionality for checkbox lists
- **TEXT EDITOR**: Adding functionality for block quotes
- **TEXT EDITOR**: Adding functionality for bold and italic text together

#### Debugging

- **IMG EDITOR**: Eraser now properly erases to white background
- **IMG EDITOR**: Cursor now no longer disappears on color picker
- **IMG EDITOR**: Cursor now no longer disappears on unsaved changes screen
- **IMG EDITOR**: Crop tool now properly resizes
- **IMG EDITOR**: Brushes, eraser, and retouch filters now no longer lag on large images
- **IMG EDITOR**: Sharpen tool no longer immediately causes artifacting
- **IMG EDITOR**: Zoom, Aspect Ratio, and Color Picker no longer are shown in Retouch tool
- **GENERAL**: Console no longer opens on build executable
- **GENERAL**: Menu version now displays cargo package version
- **GENERAL**: Can no longer click on buttons in the background with the patch notes or settings modal up

## V0.0.5

#### Features

- **JSON EDITOR**: First prototype of the Json Editor
- **JSON EDITOR**: Undo and Redo moved to top bar
- **JSON EDITOR**: Improved styling and cleaned up backgrounds
- **JSON EDITOR**: Added option to hide file info in both settings and "View" toolbar
- **JSON EDITOR**: Search bar can now search non expanded nodes
- **IMG EDITOR**: Star shape removed from brush settings
- **TEXT EDITOR**: Can now change between txt and md using file information context window (right click)
- **TEXT EDITOR**: Can now rename file in the file information context window (right click)
- **TEXT EDITOR**: Can now open file location using file information context widow (right click)
- **TEXT EDITOR**: Added the ability to load tables in the markdown mode
- **DATA CONVERTER**: First prototype of the data converter
- **GENERAL**: Added support for outputting AVIF files for the Image Editor and Image Converter
- **GENERAL**: Added easier way to add new screens/converters to the app (Code Only Change)
- **GENERAL**: Improved Light mode color scheme to be more pleasing to the eye

#### Debugging

- **JSON EDITOR**: Fixed crashing when scrolling down on large JSON's
- **JSON EDITOR**: Search now properly navigates to searched keys/values
- **JSON EDITOR**: Error popup text no longer leaves the popup
- **JSON EDITOR**: JSON Editor no longer crashes when trying to navigate 2 or more parents back
- **JSON EDITOR**: Can now undo/redo on the text view properly
- **JSON EDITOR**: No longer converts new boolean values to strings
- **IMG EDITOR**: Options bar buttons, sliders, and drag values are now centered
- **IMG EDITOR**: Preview filter button now can be turned off and on, rather than having to click cancel
- **IMG EDITOR**: Sliders and button text are now visible within light mode, slightly more hover visibility with buttons in dark mode
- **IMG EDITOR**: Fixed backgrounds and text color of some buttons within "Brush Settings" in light mode
- **IMG EDITOR**: Text box now rotates correctly on flip vertical and flip horizontal
- **IMG EDITOR**: Text box now correctly exports with wrapped words
- **TEXT EDITOR**: Code blocks and Blockquotes no longer desync on font change

## V0.0.6

#### Features
- **GENERAL**: Added right click context menu in "Recent Files" section of the sidebar
- **JSON EDITOR**: Added Line numbers to the text view
- **IMG EDITOR**: Added Layer System to the Image Editor
- **IMG EDITOR**: Color picker panel information now centered
- **IMG EDITOR**: Increased size of selection sliders
- **IMG EDITOR**: Adjusted Spray brush preset to be thicker
- **IMG EDITOR**: Adjusted Marker to display with higher default wetness
- **IMG EDITOR**: Improved performance of retouch tool filters
- **IMG EDITOR**: Custom Slider for Saturation in the retouch tool

#### Debugging
- **TEXT EDITOR**: Bolding formatting no longer makes divider line before typing
- **JSON EDITOR**: Fixed lag on large sized files within text view
- **JSON EDITOR**: Scroll "bounce" issue with large files on small display screens fixed
- **JSON EDITOR**: No longer pre-formats JSON and uses it's raw data instead
- **JSON EDITOR**: Ctrl+S save shortcut now works as intended
- **JSON EDITOR**: "Modified" and "Saved" file information status now shows up correctly
- **JSON EDITOR**: Long numbers no longer get replaced by scientific numbers
- **IMG EDITOR**: Eraser now properly erases on layers
- **IMG EDITOR**: Opacity slider now only loads when you are no longer sliding to prevent lag
- **IMG EDITOR**: Brush now no longer periodically erases background while drawing
- **IMG EDITOR**: No longer has jumpy cursor on the bottom of the color picker square
- **IMG EDITOR**: Hue slider no longer sometimes moves color square cursor
- **IMG EDITOR**: Can now scroll down on the color picker panel
- **IMG EDITOR**: Textbox no longer lags on large images
- **IMG EDITOR**: Textbox no longer allows empty textbox's (deletes layer as well)
- **IMG EDITOR**: Stamped text now matches the exact size of preview text
- **IMG EDITOR**: Background layer brush strokes no longer sometimes remove background of layer brush strokes
- **IMG EDITOR**: Drag value input no longer interferes with color picker favorites
