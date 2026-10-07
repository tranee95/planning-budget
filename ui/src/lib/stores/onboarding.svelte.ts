/**
 * Состояние знакомства: открыт ли мастер первого месяца и счётчик обновления экрана после него.
 * Мастер открывают знакомство, справка и палитра, поэтому состояние общее.
 */
class OnboardingStore {
  wizardOpen = $state(false);
  /** Меняется после записи мастера: открытый экран перечитывает данные, как после события. */
  refreshKey = $state(0);

  openWizard(): void {
    this.wizardOpen = true;
  }

  closeWizard(): void {
    this.wizardOpen = false;
  }

  /** План записан: экран под мастером пересоздаётся и показывает новые данные. */
  planApplied(): void {
    this.wizardOpen = false;
    this.refreshKey += 1;
  }
}

export const onboarding = new OnboardingStore();
