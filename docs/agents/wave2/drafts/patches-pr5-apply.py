p = 'src/session/sound_library.rs'
s = open(p).read()
a = """    pub(super) fn sound_library_on_style(&mut self, p: &mut Prepared, path: &Path) -> Pending {
        let key = patches::style_key(path);"""
assert s.count(a) == 1
s = s.replace(a, """    pub(super) fn sound_library_on_style(&mut self, p: &mut Prepared, path: &Path) -> Pending {
        // The engine may have taken over the style handed to it before this one since the
        // last pump: promote it first (the snapshot shows its tag), so this style gets the
        // bank that style is not playing, instead of reusing (rewriting) its bank.
        self.drain_snapshots();
        let key = patches::style_key(path);""")
open(p, 'w').write(s)
print("ok")
