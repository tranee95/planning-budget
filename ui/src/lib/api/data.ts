import { commands } from './bindings';
import type {
  CategoryInput,
  CardPlacementDto,
  CategoryPatchDto_Deserialize,
  ChartSpecDto,
  IncomeInput,
  IncomePatchDto_Deserialize,
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
  clearLimit: (categoryId: number, validFrom: string) =>
    call(commands.limitsClear(categoryId, validFrom)),
  limitHistory: (categoryId: number) => call(commands.limitsHistory(categoryId)),
  savingsRates: () => call(commands.savingsRatesList()),
  setSavingsRate: (categoryId: number, validFrom: string, rateBp: number) =>
    call(commands.savingsRateSet(categoryId, validFrom, rateBp)),
  setSavingsOverride: (month: string, categoryId: number, rateBp: number) =>
    call(commands.savingsOverrideSet(month, categoryId, rateBp)),
  clearSavingsOverride: (month: string, categoryId: number) =>
    call(commands.savingsOverrideClear(month, categoryId))
};

export const transactionsApi = {
  list: (month: string) => call(commands.txList(month)),
  create: (input: TransactionInput) => call(commands.txCreate(input)),
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
  create: (name: string) => call(commands.tagsCreate(name)),
  rename: (id: number, name: string) => call(commands.tagsRename(id, name)),
  remove: (id: number) => call(commands.tagsDelete(id))
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
  year: (year: number) => call(commands.summaryYear(year))
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
  transactions: (query: string, requestId: number) => call(commands.txSearch(query, requestId)),
  incomes: (query: string, requestId: number) => call(commands.incomesSearch(query, requestId))
};

export const filtersApi = {
  list: () => call(commands.filtersList()),
  save: (name: string, query: string) => call(commands.filtersSave(name, query)),
  remove: (id: number) => call(commands.filtersDelete(id))
};

export const analyticsApi = {
  run: (spec: ChartSpecDto) => call(commands.analyticsRun(spec)),
  check: (specs: ChartSpecDto[]) => call(commands.analyticsCheck(specs))
};

export const dashboardsApi = {
  list: () => call(commands.dashboardsList()),
  create: (name: string) => call(commands.dashboardsCreate(name)),
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

export const bondsApi = {
  projection: (year: number) => call(commands.bondsProjection(year))
};
