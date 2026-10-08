import { commands } from './bindings';
import type {
  CategoryInput,
  FilterScreenDto,
  CardPlacementDto,
  CategoryPatchDto_Deserialize,
  DebtInput,
  DebtPatchDto,
  DebtPaymentStatusDto,
  DebtScheduleKindDto,
  PlanWizardInputDto,
  SavingsParamsDto,
  SchedulePaymentDto,
  ChartSpecDto,
  IncomeInput,
  IncomePatchDto_Deserialize,
  SeriesRangeDto,
  SettingsPatchDto,
  TransactionInput,
  TransactionPatchDto_Deserialize,
  TxStatusDto
} from './bindings';
import { call } from './call';

export const categoriesApi = {
  list: (includeArchived = false) => call(commands.categoriesList(includeArchived)),
  create: (input: CategoryInput) => call(commands.categoriesCreate(input)),
  update: (id: number, patch: CategoryPatchDto_Deserialize) =>
    call(commands.categoriesUpdate(id, patch)),
  archive: (id: number) => call(commands.categoriesArchive(id)),
  unarchive: (id: number) => call(commands.categoriesUnarchive(id)),
  remove: (id: number) => call(commands.categoriesDelete(id)),
  reorder: (ids: number[]) => call(commands.categoriesReorder(ids)),
  setLimit: (categoryId: number, validFrom: string, amount: number) =>
    call(commands.limitsSet(categoryId, validFrom, amount)),
  unsetLimit: (categoryId: number, validFrom: string) =>
    call(commands.limitsUnset(categoryId, validFrom)),
  clearLimit: (categoryId: number, validFrom: string) =>
    call(commands.limitsClear(categoryId, validFrom)),
  limitHistory: (categoryId: number) => call(commands.limitsHistory(categoryId)),
  setSavingsRate: (categoryId: number, validFrom: string, rateBp: number) =>
    call(commands.savingsRateSet(categoryId, validFrom, rateBp)),
  setSavingsOverride: (month: string, categoryId: number, rateBp: number) =>
    call(commands.savingsOverrideSet(month, categoryId, rateBp)),
  clearSavingsOverride: (month: string, categoryId: number) =>
    call(commands.savingsOverrideClear(month, categoryId))
};

export const transactionsApi = {
  list: (month: string) => call(commands.txList(month)),
  create: (requestId: string, input: TransactionInput) => call(commands.txCreate(requestId, input)),
  update: (id: number, patch: TransactionPatchDto_Deserialize) =>
    call(commands.txUpdate(id, patch)),
  setStatus: (id: number, status: TxStatusDto) => call(commands.txSetStatus(id, status)),
  remove: (id: number) => call(commands.txDelete(id)),
  restore: (id: number) => call(commands.txRestore(id)),
  setTags: (id: number, tagIds: number[]) => call(commands.txTagsSet(id, tagIds)),
  suggest: (query: string) => call(commands.txSuggest(query)),
  categoryUsage: () => call(commands.txCategoryUsage())
};

export const tagsApi = {
  list: () => call(commands.tagsList()),
  create: (name: string) => call(commands.tagsCreate(name))
};

export const incomesApi = {
  list: (month: string) => call(commands.incomesList(month)),
  create: (input: IncomeInput) => call(commands.incomesCreate(input)),
  update: (id: number, patch: IncomePatchDto_Deserialize) =>
    call(commands.incomesUpdate(id, patch)),
  remove: (id: number) => call(commands.incomesDelete(id)),
  restore: (id: number) => call(commands.incomesRestore(id))
};

export const summaryApi = {
  month: (month: string) => call(commands.summaryMonth(month)),
  year: (year: number) => call(commands.summaryYear(year)),
  series: (month: string, range: SeriesRangeDto) => call(commands.summarySeries(month, range))
};

export const debtsApi = {
  list: (month: string, includeClosed: boolean) => call(commands.debtsList(month, includeClosed)),
  create: (input: DebtInput) => call(commands.debtsCreate(input)),
  fromTransaction: (txId: number, lender: string, schedule: SchedulePaymentDto[]) =>
    call(commands.debtFromTx(txId, lender, schedule)),
  update: (id: number, patch: DebtPatchDto) => call(commands.debtsUpdate(id, patch)),
  remove: (id: number) => call(commands.debtsDelete(id)),
  restore: (id: number) => call(commands.debtsRestore(id)),
  setPaymentStatus: (paymentId: number, status: DebtPaymentStatusDto, paidDate: string | null) =>
    call(commands.debtPaymentSetStatus(paymentId, status, paidDate)),
  schedulePreview: (amount: number, takenMonth: string, kind: DebtScheduleKindDto) =>
    call(commands.debtSchedulePreview(amount, takenMonth, kind))
};

export const planApi = {
  month: (month: string) => call(commands.planMonth(month)),
  lock: (month: string) => call(commands.planLock(month)),
  unlock: (month: string) => call(commands.planUnlock(month)),
  copyFromPrevious: (month: string) => call(commands.planCopyFromPrevious(month)),
  preview: (month: string, input: PlanWizardInputDto) => call(commands.planPreview(month, input)),
  wizardApply: (month: string, input: PlanWizardInputDto) =>
    call(commands.planWizardApply(month, input))
};

export const settingsApi = {
  get: () => call(commands.settingsGet()),
  set: (patch: SettingsPatchDto) => call(commands.settingsSet(patch))
};

/** Только dev-сборка: в релизе бэкенд отвечает ошибкой `Validation`. */
export const devApi = {
  seed: () => call(commands.devSeed())
};

/** Перенос из старой таблицы xlsx: файл выбирается в нативном диалоге, `null` — диалог закрыт. */
export const legacyApi = {
  import: () => call(commands.legacyImport())
};

export const searchApi = {
  run: (query: string, requestId: number) => call(commands.search(query, requestId)),
  /** Разбор строки запроса на токены и подсказки без выборки данных. */
  parse: (query: string) => call(commands.queryParse(query)),
  transactions: (query: string, requestId: number) => call(commands.txSearch(query, requestId)),
  incomes: (query: string, requestId: number) => call(commands.incomesSearch(query, requestId))
};

export const filtersApi = {
  list: () => call(commands.filtersList()),
  save: (name: string, query: string, screen: FilterScreenDto) =>
    call(commands.filtersSave(name, query, screen)),
  remove: (id: number) => call(commands.filtersDelete(id))
};

export const analyticsApi = {
  run: (spec: ChartSpecDto) => call(commands.analyticsRun(spec)),
  runMany: (specs: ChartSpecDto[]) => call(commands.analyticsRunMany(specs)),
  check: (specs: ChartSpecDto[]) => call(commands.analyticsCheck(specs))
};

export const dashboardsApi = {
  list: () => call(commands.dashboardsList()),
  create: (name: string) => call(commands.dashboardsCreate(name)),
  createDefault: () => call(commands.dashboardsCreateDefault()),
  rename: (id: number, name: string) => call(commands.dashboardsRename(id, name)),
  remove: (id: number) => call(commands.dashboardsDelete(id)),
  charts: (dashboardId: number) => call(commands.chartsList(dashboardId)),
  addChart: (dashboardId: number, spec: ChartSpecDto, w: number, h: number) =>
    call(commands.chartsCreate(dashboardId, spec, w, h)),
  updateChart: (id: number, spec: ChartSpecDto) => call(commands.chartsUpdate(id, spec)),
  removeChart: (id: number) => call(commands.chartsDelete(id)),
  saveLayout: (dashboardId: number, placements: CardPlacementDto[]) =>
    call(commands.chartsLayoutSet(dashboardId, placements))
};

export const savingsApi = {
  overview: (year: number) => call(commands.savingsOverview(year)),
  setParams: (categoryId: number, params: SavingsParamsDto) =>
    call(commands.savingsParamsSet(categoryId, params)),
  setFixedPlan: (categoryId: number, validFrom: string, amount: number) =>
    call(commands.savingsFixedSet(categoryId, validFrom, amount))
};
