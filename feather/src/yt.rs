use crate::{ArtistName, ChannelName, PlaylistId, PlaylistName, SongId, SongName, SongUrl};
use std::path::PathBuf;
use rustypipe::{
    client::{RustyPipe, RustyPipeQuery},
    model::MusicItem,
    param::StreamFilter,
};
use std::collections::HashMap;

/// A client for interacting with YouTube music using RustyPipe.
pub struct YoutubeClient {
    client: RustyPipeQuery,
}

impl YoutubeClient {
    /// Creates a new instance of `YoutubeClient`.
    pub fn new() -> Self {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather");
        let rp = RustyPipe::builder().storage_dir(path).build().unwrap();
        let client = rp.query();
        YoutubeClient { client }
    }

    /// Searches for music based on the given query.
    /// Returns a vector of tuples where each entry contains a song name and ID,
    /// along with a list of associated artist names.
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Vec<((SongName, SongId), Vec<ArtistName>)>, String> {
        match self.client.music_search_main(query).await {
            Ok(results) => {
                let mut search_result = vec![];

                for item in results.items.items {
                    if let MusicItem::Track(data) = item {
                        let song_id_pair = (data.name, data.id);
                        let artist_names: Vec<String> =
                            data.artists.into_iter().map(|id| id.name).collect();
                        search_result.push((song_id_pair, artist_names));
                    }
                }

                Ok(search_result)
            }
            Err(_) => Err("Error in Search Result".to_string()),
        }
    }

    /// Fetches the audio stream URL for a given song ID.
    pub async fn fetch_song_url(&self, id: &SongId) -> Result<SongUrl, String> {
        match self.client.player(&id).await {
            Ok(player) => match player.select_audio_stream(&StreamFilter::default()) {
                Some(stream) => return Ok(stream.url.clone()),
                None => return Err("Audio Stream not Found".to_string()),
            },
            Err(_) => return Err("Link cannot be Found".to_string()),
        }
    }

    /// Searches for playlists based on a given query.
    /// Returns a hashmap where the key is the playlist name and the value is a tuple
    /// containing the playlist ID and a list of associated channel names.
    pub async fn fetch_playlist(
        &self,
        search_query: &str,
    ) -> Result<HashMap<PlaylistName, (PlaylistId, Vec<ChannelName>)>, String> {
        match self.client.music_search_playlists(search_query, true).await {
            Ok(playlists) => {
                let mut result = HashMap::new();

                for playlist in playlists.items.items {
                    let playlist_id = playlist.id;
                    let channel_names: Vec<String> = playlist
                        .channel
                        .into_iter()
                        .map(|channel| channel.name)
                        .collect();

                    result.insert(playlist.name, (playlist_id, channel_names));
                }

                Ok(result)
            }
            Err(e) => Err(format!("Error in fetching playlists: {}", e)),
        }
    }

    /// Fetches songs from a given playlist ID.
    /// Returns a hashmap where each key is a tuple of (song name, song ID), and
    /// the value is a list of associated artist names.
    pub async fn fetch_playlist_songs(
        &self,
        playlist_id: PlaylistId,
    ) -> Result<HashMap<(SongName, SongId), Vec<ArtistName>>, String> {
        match self.client.playlist(playlist_id).await {
            Ok(playlist_data) => {
                let mut song_map = HashMap::new();

                for video in playlist_data.videos.items {
                    let song_key = (video.name, video.id);
                    let artist_names: Vec<String> = video
                        .channel
                        .into_iter()
                        .map(|channel| channel.name)
                        .collect();

                    song_map.insert(song_key, artist_names);
                }

                Ok(song_map)
            }
            Err(e) => Err(format!("Error fetching playlist songs: {}", e)),
        }
    }

    /// Fetches related songs for a given song ID.
    /// Returns a hashmap where each key is a tuple of (song name, song ID), and
    /// the value is a list of associated artist names.
    pub async fn fetch_related_song(
        &self,
        song_id: SongId,
    ) -> Result<HashMap<(SongName, SongId), Vec<ArtistName>>, String> {
        match self.client.music_related(song_id).await {
            Ok(music_list) => {
                let tracks = music_list.tracks;
                let mut results = HashMap::new();
                for track in tracks {
                    let song_id_name = (track.name, track.id);
                    let artist_names = track
                        .artists
                        .into_iter()
                        .map(|artist| artist.name)
                        .collect::<Vec<ArtistName>>();
                    results.insert(song_id_name, artist_names);
                }
                Ok(results)
            }
            Err(_) => Err("Error finding related songs".to_string()),
        }
    }
}
// #[tokio::test]
// async fn test_search() {
//     let client = YoutubeClient::new();

//     match client.search("Beanie").await {
//         Ok(results) => {
//             for ((song, id), artists) in results {
//                 println!("Song: {}", song);
//                 println!("Id : {}", id);
//                 println!("{}", client.fetch_song_url(id.clone()).await.unwrap());

//                 for artist in artists {
//                     println!("  - Artist: {}", artist);
//                 }
//                 test_fetch_related_song(id).await;
//                 break;
//             }
//         }
//         Err(e) => println!("Search failed: {}", e),
//     }
// }
// #[tokio::test]
// async fn test_fetch_playlist() {
//     let client = YoutubeClient::new();
//     let query = "lofi beats";

//     match client.fetch_playlist(query).await {
//         Ok(playlists) => {
//             for (playlist_name, (playlist_id, channel_names)) in playlists {
//                 println!("Playlist: {} (ID: {})", playlist_name, playlist_id);
//                 test_fetch_playlist_songs(playlist_id).await;

//                 // for channel in channel_names {
//                 //     println!("  - Channel: {}", channel);
//                 // }
//                 break;
//             }
//         }
//         Err(e) => eprintln!("Test failed: {}", e),
//     }
// }
// async fn test_fetch_playlist_songs(playlist_id :  String) {
//     let client = YoutubeClient::new();

//     match client.fetch_playlist_songs(playlist_id).await {
//         Ok(songs) => {
//             for ((song_name, song_id), artist_names) in songs {
//                 println!("Song: {} (ID: {})", song_name, song_id);
//                 let url  =  client.fetch_song_url(&song_id).await.unwrap();
//                 println!("{url:?}");
//                 for artist in artist_names {
//                     println!("  - Artist: {}", artist);
//                 }
//             }
//         }
//         Err(e) => eprintln!("Test failed: {}", e),
//     }
// }

// async fn test_fetch_related_song(song_id  :  String) {
//     let client = YoutubeClient::new();

//     match client.fetch_related_song(song_id).await {
//         Ok(related_songs) => {
//             for ((song_name, song_id), artist_names) in related_songs {
//                 println!("Related Song: {} (ID: {})", song_name, song_id);
//                 for artist in artist_names {
//                     println!("  - Artist: {}", artist);
//                 }
//             }
//         }
//         Err(e) => eprintln!("Test failed: {}", e),
//     }
// }
