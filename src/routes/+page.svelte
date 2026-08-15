<script lang="ts">
    import {invoke} from '@tauri-apps/api/core';
</script>

<h1>Local Models Orchestrators</h1>
<p><abbr title="Work In Progress">WIP</abbr></p>
<p>{await invoke('say_hello', {to: 'Carlos'})}</p>
