/**
 * Очередь сборки графиков: ECharts создаёт canvas и считает раскладку синхронно (десятки
 * миллисекунд на график), поэтому десяток карточек, собранных в один такт, дают долгую задачу и
 * рваную прокрутку. Задачи выполняются по одной за такт, между ними браузер успевает
 * отрисовать кадр и обработать ввод.
 */
type Job = { run: () => void; onError: (error: unknown) => void; cancelled: boolean };

/** Ошибка всплывает как необработанная, чтобы её увидели консоль и отчёты, но очередь идёт дальше. */
const rethrowLater = (error: unknown): void => {
  setTimeout(() => {
    throw error;
  }, 0);
};

const queue: Job[] = [];
let scheduled = false;

function schedule(): void {
  if (scheduled || queue.length === 0) return;
  scheduled = true;
  setTimeout(step, 0);
}

function step(): void {
  scheduled = false;
  const job = queue.shift();
  if (job && !job.cancelled) {
    try {
      job.run();
    } catch (error) {
      // Сбой одного графика не должен останавливать остальные.
      job.onError(error);
    }
  }
  schedule();
}

/** Ставит сборку в очередь; возвращает функцию отмены (карточка исчезла до своей очереди). */
export function enqueueInit(
  run: () => void,
  onError: (error: unknown) => void = rethrowLater
): () => void {
  const job: Job = { run, onError, cancelled: false };
  queue.push(job);
  schedule();
  return () => {
    job.cancelled = true;
  };
}

/** Сколько сборок ждёт очереди (для тестов). */
export function pendingInits(): number {
  return queue.filter((j) => !j.cancelled).length;
}
