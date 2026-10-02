<script lang="ts">
  import Combobox from './Combobox.svelte';
  import Modal from './Modal.svelte';

  let { onclose }: { onclose: () => void } = $props();
</script>

<Modal title="Форма" {onclose}>
  <Combobox
    id="c"
    label="Категория"
    value={null}
    options={[{ value: 'a', label: 'Продукты' }]}
    onchange={() => {}}
  />
</Modal>
