use super::TempDirectory;
use std::path::Path;
use std::process::{Command, Stdio};

/// A real Git repository a generated patch can be checked against.
///
/// The whole point of `getCodeReviewPatch` is that what it hands a reviewer applies. Nothing short
/// of `git apply` can say whether it does: a patch can carry the right headers, the right line
/// counts, and the right content and still be refused because a hunk's context does not match the
/// file it claims to be against. Asserting the *rendering* would only prove the renderer agrees
/// with itself.
///
/// So this builds a repository with a known base commit and known working-tree changes, and offers
/// `apply_check`, which runs the same check Git runs when a patch arrives from anywhere else.
///
/// Kept in test support rather than beside the renderer because two groups need it: 13.7-13.9
/// generate the patches and 13.13 is the gate that proves they apply and that a stale one does
/// not.
pub(crate) struct GitPatchFixture {
    directory: TempDirectory,
}

/// Why a patch was refused, or that it was not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchCheck {
    /// Git would apply this patch to the fixture as it currently stands.
    Applies,
    /// Git refused it, with what it said on stderr.
    ///
    /// The message is carried rather than discarded: "the patch did not apply" is true of a
    /// corrupt patch, a stale one, and one for a file that does not exist, and a test that could
    /// not tell them apart would pass for the wrong reason.
    Refused(String),
}

impl GitPatchFixture {
    /// A repository with one commit containing `files`, and nothing modified yet.
    ///
    /// Committed with explicit identity and no signing: a developer's global Git config is not part
    /// of the fixture, and a machine configured to sign commits would otherwise fail here for a
    /// reason that has nothing to do with what is being tested.
    pub(crate) fn committed(label: &str, files: &[(&str, &str)]) -> Self {
        let directory = TempDirectory::new(label);
        let fixture = Self { directory };
        fixture.git(&["init", "--initial-branch=main"]);
        fixture.git(&["config", "user.email", "tests@example.invalid"]);
        fixture.git(&["config", "user.name", "VaneHub Tests"]);
        fixture.git(&["config", "commit.gpgsign", "false"]);
        for (path, content) in files {
            fixture.directory.write(path, content);
        }
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "-m", "fixture base"]);
        fixture
    }

    pub(crate) fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Writes a file in the working tree without staging it, which is what a review reads.
    pub(crate) fn write(&self, relative: &str, content: &str) {
        self.directory.write(relative, content);
    }

    /// Deletes a file in the working tree without staging the deletion.
    pub(crate) fn remove(&self, relative: &str) {
        std::fs::remove_file(self.directory.path().join(relative)).expect("remove fixture file");
    }

    /// Whether Git would apply this patch to the working tree as it currently stands.
    ///
    /// `--check` and nothing else: the fixture must be readable by the next assertion in the same
    /// test, and a check that mutated the working tree would make every case after the first one
    /// depend on the order they run in.
    pub(crate) fn apply_check(&self, patch: &str) -> PatchCheck {
        self.check(patch, &[])
    }

    /// Whether Git would apply this patch to the index — the base a review diffs *from*.
    ///
    /// This is the question a review patch answers, and it is not the same one. A review patch
    /// describes committed content becoming working-tree content, so checking it against the
    /// working tree asks whether the change can be applied on top of itself: it cannot, and the
    /// refusal says "while searching for" the old lines that are no longer there. Against the
    /// index, where the committed content still is, the same patch applies.
    ///
    /// Both are kept because both are real questions. A patch generated for one snapshot and used
    /// after another has moved on fails the index check too, which is what makes staleness visible
    /// here rather than in somebody's repository.
    pub(crate) fn apply_check_cached(&self, patch: &str) -> PatchCheck {
        self.check(patch, &["--cached"])
    }

    fn check(&self, patch: &str, extra: &[&str]) -> PatchCheck {
        // Through a file rather than stdin. Git reads a patch as bytes and this crate's tests run
        // on Windows, where writing a `\n`-only patch through a pipe is one std layer away from
        // being helpfully translated into `\r\n` — which changes every hunk's content.
        let patch_path = self.directory.path().join(".vanehub-check.patch");
        std::fs::write(&patch_path, patch.as_bytes()).expect("write patch fixture");
        let output = Command::new("git")
            .current_dir(self.directory.path())
            .args(["apply", "--check", "--verbose"])
            .args(extra)
            .arg(&patch_path)
            .stdin(Stdio::null())
            .output()
            .expect("run git apply --check");
        std::fs::remove_file(&patch_path).expect("remove patch fixture");
        if output.status.success() {
            PatchCheck::Applies
        } else {
            PatchCheck::Refused(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(self.directory.path())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A patch this fixture is known to accept, written by hand so the harness is not being
    /// checked against the renderer it exists to check.
    const VALID: &str = "diff --git a/src/main.rs b/src/main.rs\n\
index 0000000..1111111 100644\n\
--- a/src/main.rs\n\
+++ b/src/main.rs\n\
@@ -1,1 +1,1 @@\n\
-fn main() {}\n\
+fn main() { work(); }\n";

    fn fixture(label: &str) -> GitPatchFixture {
        GitPatchFixture::committed(label, &[("src/main.rs", "fn main() {}\n")])
    }

    #[test]
    fn a_patch_that_matches_the_committed_content_applies() {
        assert_eq!(
            fixture("patch-applies").apply_check(VALID),
            PatchCheck::Applies
        );
    }

    // Without this the harness could report every patch as applicable and 13.13 would pass while
    // proving nothing. The three cases are the three ways a review patch goes wrong.
    #[test]
    fn a_patch_whose_context_no_longer_matches_is_refused() {
        let fixture = fixture("patch-stale-context");
        // The file moved on after the patch was generated, which is exactly the stale-witness case.
        fixture.write("src/main.rs", "fn main() { already_changed(); }\n");

        let PatchCheck::Refused(message) = fixture.apply_check(VALID) else {
            panic!("a patch against content that no longer exists must be refused");
        };
        assert!(
            message.contains("src/main.rs"),
            "the refusal must name the file: {message}"
        );
    }

    #[test]
    fn a_patch_for_a_file_that_is_not_there_is_refused() {
        let fixture = fixture("patch-missing-file");
        fixture.remove("src/main.rs");

        assert!(matches!(fixture.apply_check(VALID), PatchCheck::Refused(_)));
    }

    #[test]
    fn a_malformed_patch_is_refused_rather_than_ignored() {
        let fixture = fixture("patch-malformed");

        // Hunk header says two lines; the body has one. A renderer that miscounts produces exactly
        // this, and it is the failure a rendering-only assertion cannot see.
        let miscounted = VALID.replace("@@ -1,1 +1,1 @@", "@@ -1,2 +1,2 @@");
        assert!(matches!(
            fixture.apply_check(&miscounted),
            PatchCheck::Refused(_)
        ));
        assert!(matches!(
            fixture.apply_check("not a patch at all\n"),
            PatchCheck::Refused(_)
        ));
    }

    #[test]
    fn a_patch_can_be_checked_against_the_index_rather_than_the_working_tree() {
        let fixture = fixture("patch-cached");
        // The change is already in the working tree, which is what a review looks at. Checking
        // there asks whether it can be applied on top of itself; checking the index asks the
        // question a review patch is actually answering.
        fixture.write("src/main.rs", "fn main() { work(); }\n");

        assert!(matches!(fixture.apply_check(VALID), PatchCheck::Refused(_)));
        assert_eq!(fixture.apply_check_cached(VALID), PatchCheck::Applies);
    }

    #[test]
    fn checking_a_patch_leaves_the_working_tree_alone() {
        let fixture = fixture("patch-check-is-read-only");

        assert_eq!(fixture.apply_check(VALID), PatchCheck::Applies);

        // `--check` and not `--apply`. A check that wrote would make every later assertion in a
        // test depend on which order the checks ran in.
        let content = std::fs::read_to_string(fixture.root().join("src/main.rs")).expect("read");
        assert_eq!(content, "fn main() {}\n");
        assert!(!fixture.root().join(".vanehub-check.patch").exists());
    }
}
