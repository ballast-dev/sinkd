#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct RsyncFlags {
    pub verbose: bool,
    pub checksum: bool,
    pub archive: bool,
    pub recursive: bool,
    pub relative: bool,
    pub update: bool,
    pub inplace: bool,
    pub append: bool,
    pub append_verify: bool,
    pub dirs: bool,
    pub links: bool,
    pub copy_links: bool,
    pub copy_unsafe_links: bool,
    pub safe_links: bool,
    pub munge_links: bool,
    pub copy_dirlinks: bool,
    pub keep_dirlinks: bool,
    pub hard_links: bool,
    pub perms: bool,
    pub executability: bool,
    pub chmod: Option<String>, // --chmod takes a parameter
    pub acls: bool,
    pub xattrs: bool,
    pub owner: bool,
    pub group: bool,
    pub devices: bool,
    pub specials: bool,
    pub times: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub super_user: bool,
    pub fake_super: bool,
    pub sparse: bool,
    pub preallocate: bool,
    pub dry_run: bool,
    pub whole_file: bool,
    pub checksum_choice: Option<String>, // --checksum-choice takes a param
    pub one_file_system: bool,
    pub block_size: Option<String>, // --block-size takes a param
    pub rsh: Option<String>,        // --rsh takes a param
    pub existing: bool,
    pub ignore_existing: bool,
    pub remove_source_files: bool,
    pub delete: bool,
    pub delete_before: bool,
    pub delete_during: bool,
    pub delete_delay: bool,
    pub delete_after: bool,
    pub delete_excluded: bool,
    pub ignore_missing_args: bool,
    pub delete_missing_args: bool,
    pub ignore_errors: bool,
    pub force: bool,
    pub max_delete: Option<String>, // takes a param
    pub max_size: Option<String>,   // takes a param
    pub min_size: Option<String>,   // takes a param
    pub partial: bool,
    pub partial_dir: Option<String>, // takes a param
    pub delay_updates: bool,
    pub prune_empty_dirs: bool,
    pub numeric_ids: bool,
    pub usermap: Option<String>,  // takes a param
    pub groupmap: Option<String>, // takes a param
    pub chown: Option<String>,    // takes a param
    pub ignore_times: bool,
    pub size_only: bool,
    pub modify_window: Option<String>, // takes a param
    pub temp_dir: Option<String>,      // takes a param
    pub fuzzy: bool,
    pub compare_dest: Option<String>, // takes a param
    pub copy_dest: Option<String>,    // takes a param
    pub link_dest: Option<String>,    // takes a param
    pub compress: bool,
    pub compress_level: Option<String>, // takes a param
    pub skip_compress: Option<String>,  // takes a param
    pub cvs_exclude: bool,
    pub filter_rule: Option<String>,  // --filter takes a param
    pub exclude: Option<String>,      // --exclude takes a param
    pub exclude_from: Option<String>, // takes a param
    pub include: Option<String>,      // --include takes a param
    pub include_from: Option<String>, // takes a param
    pub files_from: Option<String>,   // takes a param
    pub from0: bool,
    pub protect_args: bool,
    pub address: Option<String>,  // takes a param
    pub port: Option<String>,     // takes a param
    pub sockopts: Option<String>, // takes a param
    pub blocking_io: bool,
    pub outbuf: Option<String>, // takes a param
    pub stats: bool,
    pub eight_bit_output: bool,
    pub partial_progress: bool, // -P (alias for --partial --progress)
    pub itemize_changes: bool,
    pub bwlimit: Option<String>,       // takes a param
    pub protocol: Option<String>,      // takes a param
    pub iconv: Option<String>,         // takes a param
    pub checksum_seed: Option<String>, // takes a param
    pub ipv4: bool,
    pub ipv6: bool,
}

#[allow(dead_code)]
impl RsyncFlags {
    /// Parse CLI arguments into RsyncFlags.
    pub fn parse<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut flags = RsyncFlags::default();

        // Helper to parse flag with optional parameter (e.g. --rsh=COMMAND).
        fn parse_kv(flag: &str) -> (String, Option<String>) {
            if let Some((key, val)) = flag.split_once('=') {
                (key.to_string(), Some(val.to_string()))
            } else {
                (flag.to_string(), None)
            }
        }

        for arg in args {
            let arg_str = arg.as_ref();
            let (key, val_opt) = parse_kv(arg_str);

            match key.as_str() {
                "-v" | "--verbose" => flags.verbose = true,
                "-c" | "--checksum" => flags.checksum = true,
                "-a" | "--archive" => flags.archive = true,
                "-r" | "--recursive" => flags.recursive = true,
                "-R" | "--relative" => flags.relative = true,
                "-u" | "--update" => flags.update = true,
                "--inplace" => flags.inplace = true,
                "--append" => flags.append = true,
                "--append-verify" => flags.append_verify = true,
                "-d" | "--dirs" => flags.dirs = true,
                "-l" | "--links" => flags.links = true,
                "-L" | "--copy-links" => flags.copy_links = true,
                "--copy-unsafe-links" => flags.copy_unsafe_links = true,
                "--safe-links" => flags.safe_links = true,
                "--munge-links" => flags.munge_links = true,
                "-k" | "--copy-dirlinks" => flags.copy_dirlinks = true,
                "-K" | "--keep-dirlinks" => flags.keep_dirlinks = true,
                "-H" | "--hard-links" => flags.hard_links = true,
                "-p" | "--perms" => flags.perms = true,
                "-E" | "--executability" => flags.executability = true,
                "--chmod" => flags.chmod = val_opt,
                "-A" | "--acls" => flags.acls = true,
                "-X" | "--xattrs" => flags.xattrs = true,
                "-o" | "--owner" => flags.owner = true,
                "-g" | "--group" => flags.group = true,
                "--devices" => flags.devices = true,
                "--specials" => flags.specials = true,
                "-t" | "--times" => flags.times = true,
                "-O" | "--omit-dir-times" => flags.omit_dir_times = true,
                "-J" | "--omit-link-times" => flags.omit_link_times = true,
                "--super" => flags.super_user = true,
                "--fake-super" => flags.fake_super = true,
                "-S" | "--sparse" => flags.sparse = true,
                "--preallocate" => flags.preallocate = true,
                "-n" | "--dry-run" => flags.dry_run = true,
                "-W" | "--whole-file" => flags.whole_file = true,
                "--checksum-choice" => flags.checksum_choice = val_opt,
                "-x" | "--one-file-system" => flags.one_file_system = true,
                "-B" | "--block-size" => flags.block_size = val_opt,
                "-e" | "--rsh" => flags.rsh = val_opt,
                "--existing" => flags.existing = true,
                "--ignore-existing" => flags.ignore_existing = true,
                "--remove-source-files" => flags.remove_source_files = true,
                "--del" | "--delete" => flags.delete = true,
                "--delete-before" => flags.delete_before = true,
                "--delete-during" => flags.delete_during = true,
                "--delete-delay" => flags.delete_delay = true,
                "--delete-after" => flags.delete_after = true,
                "--delete-excluded" => flags.delete_excluded = true,
                "--ignore-missing-args" => flags.ignore_missing_args = true,
                "--delete-missing-args" => flags.delete_missing_args = true,
                "--ignore-errors" => flags.ignore_errors = true,
                "--force" => flags.force = true,
                "--max-delete" => flags.max_delete = val_opt,
                "--max-size" => flags.max_size = val_opt,
                "--min-size" => flags.min_size = val_opt,
                "--partial" => flags.partial = true,
                "--partial-dir" => flags.partial_dir = val_opt,
                "--delay-updates" => flags.delay_updates = true,
                "-m" | "--prune-empty-dirs" => flags.prune_empty_dirs = true,
                "--numeric-ids" => flags.numeric_ids = true,
                "--usermap" => flags.usermap = val_opt,
                "--groupmap" => flags.groupmap = val_opt,
                "--chown" => flags.chown = val_opt,
                "-I" | "--ignore-times" => flags.ignore_times = true,
                "--size-only" => flags.size_only = true,
                "-@" | "--modify-window" => flags.modify_window = val_opt,
                "-T" | "--temp-dir" => flags.temp_dir = val_opt,
                "-y" | "--fuzzy" => flags.fuzzy = true,
                "--compare-dest" => flags.compare_dest = val_opt,
                "--copy-dest" => flags.copy_dest = val_opt,
                "--link-dest" => flags.link_dest = val_opt,
                "-z" | "--compress" => flags.compress = true,
                "--compress-level" => flags.compress_level = val_opt,
                "--skip-compress" => flags.skip_compress = val_opt,
                "-C" | "--cvs-exclude" => flags.cvs_exclude = true,
                "-f" | "--filter" => flags.filter_rule = val_opt,
                "--exclude" => flags.exclude = val_opt,
                "--exclude-from" => flags.exclude_from = val_opt,
                "--include" => flags.include = val_opt,
                "--include-from" => flags.include_from = val_opt,
                "--files-from" => flags.files_from = val_opt,
                "-0" | "--from0" => flags.from0 = true,
                "-s" | "--protect-args" => flags.protect_args = true,
                "--address" => flags.address = val_opt,
                "--port" => flags.port = val_opt,
                "--sockopts" => flags.sockopts = val_opt,
                "--blocking-io" => flags.blocking_io = true,
                "--outbuf" => flags.outbuf = val_opt,
                "--stats" => flags.stats = true,
                "-8" | "--8-bit-output" => flags.eight_bit_output = true,
                // -P == --partial + --progress
                "-P" => {
                    flags.partial = true;
                    flags.partial_progress = true;
                }
                "-i" | "--itemize-changes" => flags.itemize_changes = true,
                "--bwlimit" => flags.bwlimit = val_opt,
                "--protocol" => flags.protocol = val_opt,
                "--iconv" => flags.iconv = val_opt,
                "--checksum-seed" => flags.checksum_seed = val_opt,
                "-4" | "--ipv4" => flags.ipv4 = true,
                "-6" | "--ipv6" => flags.ipv6 = true,
                _ => {
                    // Ignore unknown or unmatched flags,
                    // or handle error/logging if desired.
                }
            }
        }

        flags
    }

    /// Collect all *active* flags into a Vec<String>.
    pub fn get(&self) -> Vec<String> {
        let mut result = Vec::new();

        // For brevity, just a few samples shown. Expand similarly for all flags:
        if self.verbose {
            result.push("--verbose".to_string());
        }
        if self.checksum {
            result.push("--checksum".to_string());
        }
        if self.archive {
            result.push("--archive".to_string());
        }
        if self.ipv4 {
            result.push("--ipv4".to_string());
        }
        if self.ipv6 {
            result.push("--ipv6".to_string());
        }
        // ... continue for all other fields ...

        // Example of pushing a flag with a value (if set):
        if let Some(ref block_size) = self.block_size {
            result.push(format!("--block-size={}", block_size));
        }

        result
    }
}

// ----- Usage Example -----
// fn main() {
//     let args = vec!["-v", "--checksum", "--block-size=4096", "-4"];
//     let flags = RsyncFlags::parse(args);
//     let activated = flags.get();
//     println!("Activated flags: {:?}", activated);
// }

/* -- supported rsync flags --

-v, --verbose               increase verbosity
-c, --checksum              skip based on checksum, not mod-time & size
-a, --archive               archive mode; equals -rlptgoD (no -H,-A,-X)
    --no-OPTION             turn off an implied OPTION (e.g. --no-D)
-r, --recursive             recurse into directories
-R, --relative              use relative path names
    --no-implied-dirs       don't send implied dirs with --relative
-u, --update                skip files that are newer on the receiver
    --inplace               update destination files in-place
    --append                append data onto shorter files
    --append-verify         --append w/old data in file checksum
-d, --dirs                  transfer directories without recursing
-l, --links                 copy symlinks as symlinks
-L, --copy-links            transform symlink into referent file/dir
    --copy-unsafe-links     only "unsafe" symlinks are transformed
    --safe-links            ignore symlinks that point outside the tree
    --munge-links           munge symlinks to make them safer
-k, --copy-dirlinks         transform symlink to dir into referent dir
-K, --keep-dirlinks         treat symlinked dir on receiver as dir
-H, --hard-links            preserve hard links
-p, --perms                 preserve permissions
-E, --executability         preserve executability
    --chmod=CHMOD           affect file and/or directory permissions
-A, --acls                  preserve ACLs (implies -p)
-X, --xattrs                preserve extended attributes
-o, --owner                 preserve owner (super-user only)
-g, --group                 preserve group
    --devices               preserve device files (super-user only)
    --specials              preserve special files
-D                          same as --devices --specials
-t, --times                 preserve modification times
-O, --omit-dir-times        omit directories from --times
-J, --omit-link-times       omit symlinks from --times
    --super                 receiver attempts super-user activities
    --fake-super            store/recover privileged attrs using xattrs
-S, --sparse                turn sequences of nulls into sparse blocks
    --preallocate           allocate dest files before writing
-n, --dry-run               perform a trial run with no changes made
-W, --whole-file            copy files whole (w/o delta-xfer algorithm)
    --checksum-choice=STR   choose the checksum algorithms
-x, --one-file-system       don't cross filesystem boundaries
-B, --block-size=SIZE       force a fixed checksum block-size
-e, --rsh=COMMAND           specify the remote shell to use
    --existing              skip creating new files on receiver
    --ignore-existing       skip updating files that exist on receiver
    --remove-source-files   sender removes synchronized files (non-dir)
    --del                   an alias for --delete-during
    --delete                delete extraneous files from dest dirs
    --delete-before         receiver deletes before xfer, not during
    --delete-during         receiver deletes during the transfer
    --delete-delay          find deletions during, delete after
    --delete-after          receiver deletes after transfer, not during
    --delete-excluded       also delete excluded files from dest dirs
    --ignore-missing-args   ignore missing source args without error
    --delete-missing-args   delete missing source args from destination
    --ignore-errors         delete even if there are I/O errors
    --force                 force deletion of dirs even if not empty
    --max-delete=NUM        don't delete more than NUM files
    --max-size=SIZE         don't transfer any file larger than SIZE
    --min-size=SIZE         don't transfer any file smaller than SIZE
    --partial               keep partially transferred files
    --partial-dir=DIR       put a partially transferred file into DIR
    --delay-updates         put all updated files into place at end
-m, --prune-empty-dirs      prune empty directory chains from file-list
    --numeric-ids           don't map uid/gid values by user/group name
    --usermap=STRING        custom username mapping
    --groupmap=STRING       custom groupname mapping
    --chown=USER:GROUP      simple username/groupname mapping
-I, --ignore-times          don't skip files that match size and time
    --size-only             skip files that match in size
-@, --modify-window=NUM     set the accuracy for mod-time comparisons
-T, --temp-dir=DIR          create temporary files in directory DIR
-y, --fuzzy                 find similar file for basis if no dest file
    --compare-dest=DIR      also compare received files relative to DIR
    --copy-dest=DIR         ... and include copies of unchanged files
    --link-dest=DIR         hardlink to files in DIR when unchanged
-z, --compress              compress file data during the transfer
    --compress-level=NUM    explicitly set compression level
    --skip-compress=LIST    skip compressing files with suffix in LIST
-C, --cvs-exclude           auto-ignore files in the same way CVS does
-f, --filter=RULE           add a file-filtering RULE
-F                          same as --filter='dir-merge /.rsync-filter'
                            repeated: --filter='- .rsync-filter'
    --exclude=PATTERN       exclude files matching PATTERN
    --exclude-from=FILE     read exclude patterns from FILE
    --include=PATTERN       don't exclude files matching PATTERN
    --include-from=FILE     read include patterns from FILE
    --files-from=FILE       read list of source-file names from FILE
-0, --from0                 all *from/filter files are delimited by 0s
-s, --protect-args          no space-splitting; wildcard chars only
    --address=ADDRESS       bind address for outgoing socket to daemon
    --port=PORT             specify double-colon alternate port number
    --sockopts=OPTIONS      specify custom TCP options
    --blocking-io           use blocking I/O for the remote shell
    --outbuf=N|L|B          set out buffering to None, Line, or Block
    --stats                 give some file-transfer stats
-8, --8-bit-output          leave high-bit chars unescaped in output
-P                          same as --partial --progress
-i, --itemize-changes       output a change-summary for all updates
    --bwlimit=RATE          limit socket I/O bandwidth
    --protocol=NUM          force an older protocol version to be used
    --iconv=CONVERT_SPEC    request charset conversion of filenames
    --checksum-seed=NUM     set block/file checksum seed (advanced)
-4, --ipv4                  prefer IPv4
-6, --ipv6                  prefer IPv6

*/
