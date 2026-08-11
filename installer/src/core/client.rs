pub const CLIENT_NAME: &str = "ChromieCraft 3.3.5a";
pub const WOW_EXE: &str = "WoW.exe";

/// Direct HTTP(S) mirror of the client zip, used when BitTorrent is blocked
/// or the swarm is unreachable. No SHA-256 is known for the client archive,
/// so this download is trusted without a checksum.
pub const CLIENT_HTTP_URL: &str = "https://btground.dedyn.io/chmi/ChromieCraft_3.3.5a.zip";

/// ChromieCraft 3.3.5a client magnet link.
///
/// A mix of UDP and HTTP trackers is included on purpose: on networks where
/// UDP is blocked (common on Windows), the HTTP trackers still announce peers
/// and resolve metadata over TCP.
pub const CLIENT_MAGNET: &str = "magnet:?xt=urn:btih:2ba2833baf733ce0a16040d43ed09491f2bf2ab2&dn=ChromieCraft_3.3.5a.zip&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=http%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.uw0.xyz%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.zerobytes.xyz%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Fexodus.desync.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=http%3A%2F%2Fp4p.arenabg.com%3A1337%2Fannounce&tr=http%3A%2F%2Ftracker.gbitt.info%3A80%2Fannounce";
