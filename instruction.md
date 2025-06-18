# Feather - YouTube Music TUI Testing Instructions

## Installation

### Prerequisites
- **Rust** (latest stable version)
- **MPV** media player
- **Git**

### Installing Feather

1. **Clone the repository:**
   ```bash
   git clone --branch v0.2 --depth 1 https://github.com/13unk0wn/Feather.git
   cd Feather
   ```

2. **Build the application:**
   ```bash
   cargo build --release
   ```

3. **Run the application:**
   ```bash
   ./target/release/feather_frontend
   ```

## How to Use Feather

### Basic Navigation
Feather uses a leader key system (default: `:`) followed by navigation keys:

| Key Combination | Action |
|----------------|--------|
| `:q` | Quit application |
| `:s` | Open Search Tab |
| `:h` | Open History Tab |
| `:p` | Open Player Tab |
| `:u` | Open User Playlist Tab |

### Search Mode (`:s`)

#### Switching Between Search Types
- `;` - Switch between Search Songs and Search Playlists

#### Search Songs Mode
- `[TAB]` - Switch between Search Input and Search Results

**Search Input:**
- Type your search query
- `Enter` - Perform search
- `Shift + Insert` - Paste from clipboard

**Search Results:**
- `Arrow Up / k` - Navigate up
- `Arrow Down / j` - Navigate down
- `Enter` - Play selected song
- `a` - Add song to playlist

#### Playlist Search Mode
- `[` - Switch between Playlist Search List and View Selected Playlist

**View Selected Playlist:**
- `p` - Play entire playlist (auto-play enabled)
- `Enter` - Play from specific song in playlist

### History Mode (`:h`)
- `Arrow Up / k` - Navigate up
- `Arrow Down / j` - Navigate down
- `a` - Add song to playlist

### User Playlist Mode (`:u`)
- `` ` `` - Create new playlist (backtick key)
- Same navigation as Playlist Search Mode
- Songs can be added from History and Search results

### Player Mode (`:p`)
- `n` - Skip to next song
- `p` - Skip to previous song
- `Arrow Up/Down` - Adjust volume (±5)
- `Arrow Left/Right` - Seek forward/backward (±5 seconds)
- `Space` - Pause/Resume playback

## Features

### 🎵 Music Playback
- **YouTube Music Integration**: Search and play music directly from YouTube
- **MPV Backend**: High-quality audio playback using MPV player
- **Playback Controls**: Play, pause, skip, seek, and volume control
- **Auto-play**: Continuous playback with automatic song progression

### 🔍 Search Functionality
- **Song Search**: Find individual tracks by title, artist, or keywords
- **Playlist Search**: Discover and play entire playlists
- **Real-time Results**: Instant search results as you type
- **Clipboard Integration**: Easy pasting of search queries

### 📚 Playlist Management
- **Create Playlists**: Build your own music collections
- **Add Songs**: Add tracks from search results or history
- **Playlist Playback**: Play entire playlists with auto-advance
- **User Playlists**: Persistent storage of your custom playlists

### 📖 History Tracking
- **Playback History**: Automatic tracking of played songs
- **History Management**: Navigate through your listening history

### 🎨 User Interface
- **TUI Interface**: Terminal-based user interface
- **Customizable Colors**: Configurable color schemes

### ⚙️ Configuration
- **Custom Keybindings**: Remap controls to your preference
- **Color Themes**: Personalize the visual appearance

## Advanced Features

### Environment Variables
- `FEATHER_COOKIES` - Set path to YouTube cookies file for enhanced functionality

### Configuration Files
The application creates configuration files in the user's home directory for:
- Key bindings
- Color schemes


### Configuration Location
All configuration files are stored in: `~/.local/share/Feather/`

#### Modifying Configuration
You can customize Feather's behavior by editing the configuration files:

1. **Navigate to the config directory:**
   ```bash
   cd ~/.local/share/Feather/
   ```

2. **Available configuration files:**
   - `config.json` - Main application settings (colors, UI preferences)
   - `keybindings.json` - Custom key mappings
   - `playlists/` - Directory containing user playlists
   - `history.bin` - Playback history data

3. **Example configuration modifications:**
   ```json
   {
     "bg_color": [29, 32, 33],
     "text_color": [235, 219, 178],
     "selected_tab_color": [250, 189, 47],
     "play_icon": "▶",
     "pause_icon": "❚❚"
   }
   ```

4. **Backup your configuration:**
   ```bash
   cp -r ~/.local/share/Feather/ ~/.local/share/Feather_backup/
   ```

**Note:** Always backup your configuration before making changes, as incorrect modifications may cause the application to fail to start.

## Testing Guidelines

### Basic Functionality Testing
1. **Installation Test**
   - Verify successful compilation
   - Confirm MPV dependency is satisfied
   - Test application startup

2. **Navigation Testing**
   - Test all leader key combinations
   - Verify tab switching works correctly
   - Confirm keyboard shortcuts function properly

3. **Search Testing**
   - Test song search functionality
   - Verify playlist search works
   - Test clipboard paste functionality
   - Confirm search results display correctly

4. **Playback Testing**
   - Test song playback
   - Verify player controls (play, pause, skip, seek)
   - Test volume adjustment
   - Confirm auto-play functionality

5. **Playlist Testing**
   - Test playlist creation
   - Verify adding songs to playlists
   - Test playlist playback
   - Confirm playlist persistence

### Error Handling Testing
- Verify handling of invalid search queries
- Test with corrupted configuration files
- Confirm graceful handling of MPV errors

### Performance Testing
- Test with large search results
- Verify memory usage during extended playback
- Test application responsiveness during heavy operations

## Reporting Bugs

### GitHub Issues
If you encounter any bugs or issues while testing Feather, please report them on GitHub:

1. **Visit the GitHub repository**: https://github.com/13unk0wn/Feather
2. **Click on "Issues" tab**
3. **Click "New Issue"**


### Bug Report Information
When reporting bugs, please include:

- **Operating System**: Linux distribution and version
- **Feather Version**: Branch and commit hash
- **Rust Version**: `rustc --version`
- **MPV Version**: `mpv --version`
- **Steps to Reproduce**: Detailed steps to trigger the bug
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Error Messages**: Any error output or logs
- **Screenshots**: If applicable (terminal screenshots)

## Contributing

If you'd like to contribute to Feather:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Support

For additional support:
- **Report  on this thread**


---
**Testing Period - 2 weeks**

**Happy Testing! 🎵**


