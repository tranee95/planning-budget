/**
 * Скользящий индикатор выбранного пункта (`Segmented`, `Tabs`): элемент `.thumb` внутри группы
 * плавно переезжает под активную кнопку. Положение берётся из разметки (`aria-checked` или
 * `aria-selected`), поэтому компоненту не нужно знать, где стоит кнопка.
 * Первая расстановка идёт без перехода, иначе индикатор «выезжал» бы из угла при открытии.
 */
const MOVE =
  'transform var(--dur-base) var(--ease-out), width var(--dur-base) var(--ease-out), height var(--dur-base) var(--ease-out)';

export function indicator(group: HTMLElement): { destroy: () => void } {
  const thumb = group.querySelector<HTMLElement>(':scope > .thumb');
  if (!thumb) return { destroy: () => undefined };

  const place = (): void => {
    const active = group.querySelector<HTMLElement>(
      ':scope > [aria-checked="true"], :scope > [aria-selected="true"]'
    );
    if (!active) {
      thumb.style.opacity = '0';
      return;
    }
    thumb.style.opacity = '1';
    thumb.style.width = `${String(active.offsetWidth)}px`;
    thumb.style.height = `${String(active.offsetHeight)}px`;
    thumb.style.transform = `translate(${String(active.offsetLeft)}px, ${String(active.offsetTop)}px)`;
  };

  const animated = (on: boolean): void => {
    thumb.style.transition = on ? MOVE : 'none';
  };

  animated(false);
  place();
  let settleFrame = requestAnimationFrame(() => {
    animated(true);
  });
  const mutations = new MutationObserver(place);
  mutations.observe(group, {
    attributes: true,
    attributeFilter: ['aria-checked', 'aria-selected'],
    subtree: true
  });
  // Меняется состав или подпись пунктов: переставляем индикатор без перехода, он не должен отставать.
  const content = new MutationObserver(() => {
    animated(false);
    place();
    settleFrame = requestAnimationFrame(() => {
      animated(true);
    });
  });
  content.observe(group, { childList: true, characterData: true, subtree: true });
  const resizes = new ResizeObserver(() => {
    // Изменение размера двигает кнопки сразу: переход тут только отставал бы от них.
    animated(false);
    place();
    settleFrame = requestAnimationFrame(() => {
      animated(true);
    });
  });
  resizes.observe(group);
  // Ширина кнопки меняется при смене подписи или шрифта, а ширина самой группы — нет.
  for (const button of group.querySelectorAll(':scope > [role]')) resizes.observe(button);

  return {
    destroy() {
      cancelAnimationFrame(settleFrame);
      mutations.disconnect();
      content.disconnect();
      resizes.disconnect();
    }
  };
}
