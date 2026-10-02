<script lang="ts">
  import { Button, Card } from '$lib/components';
</script>

<Card title="Брокерский счёт — скоро">
  <p>
    Здесь появятся портфель и выплаченные купоны с брокерского счёта. Подключение потребует сети и
    отдельной защиты токена, поэтому в этой версии приложение остаётся полностью локальным.
  </p>
  <div>
    <Button variant="secondary" disabled>Подключить брокерский счёт</Button>
  </div>
</Card>

<style>
  p {
    margin: 0;
    color: var(--muted);
    font-size: var(--fs-14);
  }
</style>
