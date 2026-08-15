import io

path = 'src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs'
s = io.open(path, encoding='utf-8').read()


def sub(old, new, expected=1):
    global s
    found = s.count(old)
    assert found == expected, 'expected %d of %r, found %d' % (expected, old[:70], found)
    s = s.replace(old, new)


# Both the adapter struct and the generation function declare these together; add the Artifact
# store beside the repository in each, following `native_tool_operations` exactly.
sub('''    native_tool_operations: Option<Arc<SqliteNativeToolRepository>>,
    native_tool_events: Option<tauri::AppHandle>,''',
    '''    native_tool_operations: Option<Arc<SqliteNativeToolRepository>>,
    artifacts: Option<Arc<ArtifactService>>,
    native_tool_events: Option<tauri::AppHandle>,''', 2)

sub('''            native_tool_operations: None,''',
    '''            native_tool_operations: None,
            artifacts: None,''')

sub('''    pub(crate) fn with_native_tool_operations(''',
    '''    /// Supplies the Artifact store the tool loop reads an image from when a native tool names one
    /// in its result metadata (`add-onepiece-visual-tool-returns`).
    pub(crate) fn with_artifacts(mut self, artifacts: Arc<ArtifactService>) -> Self {
        self.artifacts = Some(artifacts);
        self
    }

    pub(crate) fn with_native_tool_operations(''')

sub('''        let native_tool_operations = self.native_tool_operations.clone();''',
    '''        let native_tool_operations = self.native_tool_operations.clone();
        let artifacts = self.artifacts.clone();''')

sub('''                native_tool_operations,
''', '''                native_tool_operations,
                artifacts,
''', 1)

sub('''        native_tool_operations.as_deref(),''',
    '''        native_tool_operations.as_deref(),
        artifacts.as_deref(),''')

io.open(path, 'w', encoding='utf-8').write(s)
print('plumbed artifacts through the adapter')
