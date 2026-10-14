/**
 * Признак «страница прокручивается» на корне документа (`data-scrolling`). Пока он стоит, canvas
 * графиков не принимает указатель (см. `Chart.svelte`): иначе под неподвижным курсором при каждом
 * шаге прокрутки ECharts перерисовывает подсветку и подсказку, и кадры рвутся.
 */
export function trackScrolling(doc: Document = document, idleMs = 120): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const root = doc.documentElement;
  const onScroll = (): void => {
    root.dataset.scrolling = '1';
    clearTimeout(timer);
    timer = setTimeout(() => {
      delete root.dataset.scrolling;
    }, idleMs);
  };
  const target = doc.defaultView ?? window;
  target.addEventListener('scroll', onScroll, { passive: true, capture: true });
  return () => {
    target.removeEventListener('scroll', onScroll, { capture: true });
    clearTimeout(timer);
    delete root.dataset.scrolling;
  };
}
