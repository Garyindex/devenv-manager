<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { onMount } from 'svelte'
  import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down'
  import Bot from '@lucide/svelte/icons/bot'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ClipboardList from '@lucide/svelte/icons/clipboard-list'
  import Copy from '@lucide/svelte/icons/copy'
  import Database from '@lucide/svelte/icons/database'
  import Download from '@lucide/svelte/icons/download'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import FolderOpen from '@lucide/svelte/icons/folder-open'
  import FolderSearch from '@lucide/svelte/icons/folder-search'
  import Gauge from '@lucide/svelte/icons/gauge'
  import Globe from '@lucide/svelte/icons/globe'
  import HardDrive from '@lucide/svelte/icons/hard-drive'
  import Heart from '@lucide/svelte/icons/heart'
  import Info from '@lucide/svelte/icons/info'
  import LaptopMinimalCheck from '@lucide/svelte/icons/laptop-minimal-check'
  import Languages from '@lucide/svelte/icons/languages'
  import Moon from '@lucide/svelte/icons/moon'
  import PackageSearch from '@lucide/svelte/icons/package-search'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw'
  import Route from '@lucide/svelte/icons/route'
  import ScanSearch from '@lucide/svelte/icons/scan-search'
  import Settings from '@lucide/svelte/icons/settings'
  import ShieldCheck from '@lucide/svelte/icons/shield-check'
  import Sun from '@lucide/svelte/icons/sun'
  import Terminal from '@lucide/svelte/icons/terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import Wrench from '@lucide/svelte/icons/wrench'
  import { siGithub } from 'simple-icons'
  import donationQr from './assets/donation-qr.jpg'

  type Locale = 'zh-CN' | 'en-US'
  type Risk = 'keep' | 'caution' | 'cleanable'
  type ScanMode = 'drive' | 'full'
  type Category = 'runtime' | 'cache' | 'config' | 'temp' | 'toolchain' | 'packageManager' | 'aiTool' | 'ide' | 'container' | 'mobile' | 'buildTool' | 'drive'
  type FocusView = 'overview' | 'cleanup' | 'issues' | 'toolchains' | 'packages' | 'ai' | 'large'
  type SortMode = 'sizeDesc' | 'nameAsc' | 'riskDesc' | 'status'
  type ThemeMode = 'mist' | 'graphite' | 'azure' | 'github' | 'notion' | 'dark'
  type PageMode = 'main' | 'deep'
  type SettingsTab = 'settings' | 'about' | 'updates' | 'audit' | 'donate'
  type EnvironmentTab = 'recommended' | 'missing' | 'installed'

  interface EnvEntry {
    id: string
    nameKey: string
    path: string
    category: Category
    risk: Risk
    sizeBytes: number
    exists: boolean
    descriptionKey: string
    modified?: string
    fileCount: number
    dirCount: number
    sizeApproximate: boolean
  }

  interface PathEntry { path: string; exists: boolean; source: string }
  interface ScanReport {
    homeDir: string
    platform: string
    totalSizeBytes: number
    entries: EnvEntry[]
    pathEntries: PathEntry[]
    scanMode: ScanMode
    elapsedMs: number
    estimatedMemoryBytes: number
  }
  interface SizeEstimate { bytes: number; files: number; dirs: number; truncated: boolean }
  interface DeepScanReport { estimate: SizeEstimate; children: EnvEntry[] }
  interface ScanProgress { entry: EnvEntry; scanned: number; total: number }
  interface ToolInsights {
    cleanableCount: number
    toolchainCount: number
    aiToolCount: number
    brokenPathCount: number
    configRuleCount: number
    dependencyEdgeCount: number
  }
  interface CleanupPreview { path: string; allowed: boolean; risk: Risk; estimate: SizeEstimate; reason: string }
  interface QuarantineItem { id: string; originalPath: string; quarantinePath: string; created: string }
  interface AuditLogEntry { id: string; action: string; targetPath: string; status: string; detail: string; created: string }
  interface ScrollDragState { pointerId: number; startX: number; startY: number; scrollLeft: number; scrollTop: number; active: boolean }
  interface InstallVersionOption { id: string; label: string; packageId: string; command: string[]; needsAdmin: boolean; sourceId: string; manager: string; qualityScore: number; downloadUrl?: string }
  interface InstallPlan { packageManager: string; packageId: string; command: string[]; needsAdmin: boolean; notes: string; sourceId: string; sourceQuality: number; downloadUrl?: string }
  interface ToolDescriptions { short?: string; long?: string; source?: string; homepage?: string; lastUpdatedAt?: string }
  interface ToolUsageMetadata { primaryUseCases: string[]; keywords: string[]; relatedTools: string[] }
  interface ToolNotesMetadata { install: string[]; upgrade: string[]; knownIssues: string[] }
  interface ToolLinks { homepage?: string; download?: string; releases?: string; docs?: string }
  interface ToolQualityMetadata { confidence: string; score: number; official: boolean; lastSuccessfulScanAt?: string; failureCount: number; staleAfterDays?: number }
  interface ToolVerifyCommand { command: string; args: string[]; expectedPattern: string }
  interface ToolLifecycleMetadata { status?: string; deprecated: boolean; replacedBy?: string }
  interface ToolDependenciesMetadata { required: boolean; requiredBySuites: string[]; dependsOn: string[]; optionalDependencies: string[] }
  interface ToolRiskMetadata { requiresAdmin: boolean; portable: boolean; systemCritical: boolean }
  interface ToolScanMetadata { status: string; scannedAt?: string; errors: string[] }
  interface ToolPackageMetadata { name?: string; publisher?: string; author?: string; description?: string; license?: string; licenseUrl?: string; tags: string[] }
  interface ToolVersionDetail { version: string; channel: string; latest: boolean; prerelease: boolean; lts?: boolean; releaseDate?: string; eolDate?: string; changelogUrl?: string }
  interface ToolDownloadDetail { id: string; version?: string; platform: string; architecture?: string; kind: string; url?: string; urlType: string; direct: boolean; sha256?: string; sizeBytes?: number }
  interface ToolCommandDetail { action: string; manager: string; platform: string; version?: string; requiresAdmin: boolean; supportsVersion: boolean; shell: string; command: string[]; template: string[] }
  interface ToolSourceDetail {
    id: string
    manager: string
    packageId?: string
    official: boolean
    priority: number
    platforms: string[]
    scan: ToolScanMetadata
    package: ToolPackageMetadata
    descriptions: ToolDescriptions
    usage: ToolUsageMetadata
    notes: ToolNotesMetadata
    links: ToolLinks
    quality: ToolQualityMetadata
    versions: ToolVersionDetail[]
    downloads: ToolDownloadDetail[]
    commands: ToolCommandDetail[]
    verifyCommands: ToolVerifyCommand[]
  }
  interface ToolDetailMetadata {
    aliases: string[]
    tags: string[]
    platforms: string[]
    lifecycle: ToolLifecycleMetadata
    descriptions: ToolDescriptions
    usage: ToolUsageMetadata
    notes: ToolNotesMetadata
    links: ToolLinks
    dependencies: ToolDependenciesMetadata
    risk: ToolRiskMetadata
    quality: ToolQualityMetadata
    verifyCommands: ToolVerifyCommand[]
    sources: ToolSourceDetail[]
  }
  interface DevToolCheck {
    id: string
    name: string
    summary?: string
    description?: string
    command: string
    installed: boolean
    version?: string
    source: string
    status: 'pending' | 'installed' | 'missing'
    required: boolean
    category: string
    installPlan?: InstallPlan
    uninstallPlan?: InstallPlan
    versionOptions: InstallVersionOption[]
    detail?: ToolDetailMetadata
  }
  interface EnvironmentSuite {
    id: string
    name: string
    description: string
    toolIds: string[]
    requiredCount: number
    installedCount: number
    missingCount: number
    adminRequired: boolean
  }
  interface EnvironmentCheckReport {
    platform: string
    packageManagerAvailable: boolean
    packageManagerVersion?: string
    packageManagerChecked: boolean
    requiredMissing: number
    optionalMissing: number
    checkedTools: number
    pendingTools: number
    totalTools: number
    completed: boolean
    tools: DevToolCheck[]
    suites: EnvironmentSuite[]
  }
  interface EnvironmentCheckProgress { report: EnvironmentCheckReport; checked: number; total: number; completed: boolean }
  interface InstallResult { toolId: string; status: string; exitCode?: number; output: string }
  interface AppUpdateInfo {
    currentVersion: string
    latestVersion?: string
    updateAvailable: boolean
    releaseUrl: string
    downloadUrl?: string
    assetName?: string
    publishedAt?: string
    body?: string
    error?: string
  }
  interface AppMetadata { currentVersion: string; githubUrl: string; releasesUrl: string; officialUrl: string }

  const zh = {
    title: '开发环境管家',
    subtitle: '轻量扫描程序员环境，先看到大小、风险和可清理项，再决定深扫或隔离。',
    scan: '扫描',
    rescan: '重新扫描',
    export: '导出',
    settings: '设置',
    back: '返回',
    theme: '主题',
    themeStyle: '配色风格',
    themeMist: 'Apple 灰蓝',
    themeGraphite: '石墨灰',
    themeAzure: 'Microsoft 蓝',
    themeGithub: 'GitHub 中性',
    themeNotion: 'Notion 墨白',
    lightTheme: '浅色',
    darkTheme: '深色',
    deepPage: '深度扫描',
    deepTarget: '扫描目标',
    deepSummary: '深扫摘要',
    childRanking: '子项排行',
    quickActions: '快捷操作',
    openFolder: '打开目录',
    focusView: '专项视图',
    batchActions: '批量操作',
    diagnostics: '诊断',
    auditLog: '操作日志',
    environment: '环境体检',
    mainPage: '环境扫描',
    environmentPage: '环境体检',
    checkEnvironment: '检查环境',
    environmentIntro: '检查开发工具是否已安装，并为缺失项生成可确认的补齐方案。',
    quickInstall: '快捷安装',
    installable: '可补齐',
    checkedTools: '工具明细',
    packageManagerStatus: '包管理器',
    requiredMissing: '必需缺失',
    optionalMissing: '可选缺失',
    installed: '已安装',
    missingTool: '未安装',
    install: '补齐安装',
    installing: '安装中',
    installPlan: '安装计划',
    environmentSuites: '环境套件',
    installSuite: '补齐套件',
    suiteComplete: '已完整',
    suiteMissing: '缺失',
    needsAdmin: '可能需要管理员权限',
    coreRequired: '必需',
    optional: '可选',
    noInstallPlan: '当前平台未提供自动安装方案。',
    noInstallPlanShort: '无方案',
    installConfirm: '确认安装',
    installResult: '安装结果',
    uninstall: '一键卸载',
    uninstalling: '卸载中',
    uninstallConfirm: '确认卸载',
    uninstallCommand: '卸载指令',
    uninstallSafetyHint: '将执行上方卸载指令。再次确认后才会启动卸载。',
    uninstallDetailOnly: '卸载只能在工具详情页执行。',
    noUninstallPlan: '当前来源未提供自动卸载方案。',
    cancel: '取消',
    openTerminal: '打开终端',
    copyCommand: '复制命令',
    version: '版本',
    reinstall: '重装/切换版本',
    openDetails: '查看详情',
    toolSource: '来源',
    installCommand: '安装指令',
    detectedVersion: '检测版本',
    selectedVersion: '选择版本',
    metadataOverview: '元数据概览',
    sourceDetails: '安装源详情',
    links: '链接',
    homepage: '官网',
    docs: '文档',
    usage: '用途',
    keywords: '关键词',
    primaryUseCases: '主要用途',
    relatedTools: '相关工具',
    notes: '备注',
    installNotes: '安装备注',
    upgradeNotes: '升级备注',
    knownIssues: '已知问题',
    verifyCommands: '验证命令',
    downloads: '下载项',
    publisher: '发布者',
    license: '许可证',
    qualityScore: '质量评分',
    confidence: '可信度',
    scanStatus: '扫描状态',
    scannedAt: '扫描时间',
    versions: '历史版本',
    latest: '最新',
    prerelease: '预览版',
    releaseDate: '发布日期',
    eolDate: '停止维护',
    changelog: '更新日志',
    packageId: '包 ID',
    officialSource: '官方源',
    directDownload: '直链',
    noMetadata: '暂无更多元数据。',
    commandTemplate: '命令模板',
    lifecycle: '生命周期',
    riskInfo: '风险信息',
    recommendedInstall: '建议安装',
    recommendedInstallIntro: '优先补齐必需项，以及当前平台已内置安装方案的常用开发工具。',
    missingEnvironments: '未安装环境',
    installedEnvironments: '已安装环境',
    pendingEnvironments: '检查中',
    noRecommendedInstall: '暂无建议安装项。',
    noMissingEnvironments: '全部环境已安装或暂未发现缺失项。',
    noInstalledEnvironments: '暂未检测到已安装环境。',
    environmentChecking: '正在检查环境',
    environmentProgress: '已检查',
    sizeLoading: '正在加载实际大小',
    openEnvironmentWindow: '打开环境体检',
    ruleCoverage: '规则覆盖',
    scanScope: '扫描范围',
    scanDrive: '扫描盘符',
    pendingScan: '待深扫',
    refreshSize: '刷新大小',
    safetyPreview: '安全预览',
    estimatedFiles: '文件',
    estimatedDirs: '目录',
    noAuditLog: '暂无操作日志。',
    sortBy: '排序',
    sortSize: '按大小',
    sortName: '按名称',
    sortRisk: '按风险',
    sortStatus: '按状态',
    selectCleanable: '选择可清理',
    clearSelection: '清空选择',
    overview: '总览',
    cleanupView: '清理专项',
    issuesView: '问题专项',
    toolchainView: '工具链',
    packageView: '包管理',
    aiView: 'AI 工具',
    largeView: '大体积',
    smartDeepScan: '智能深扫',
    actionPlan: '行动建议',
    noPathIssues: 'PATH 暂未发现异常。',
    noPlan: '暂无建议，扫描结果比较干净。',
    about: '关于',
    updateCheck: '版本检查',
    donate: '打赏',
    close: '关闭',
    appVersion: '当前版本',
    buildType: '构建类型',
    portableBuild: '便携版 / 安装包',
    languageSetting: '界面语言',
    updateStatus: '通过 GitHub Releases 检查最新安装包，找到可用版本后可打开发布页或下载入口。',
    checkNow: '立即检查',
    checkingUpdate: '检查中',
    officialWebsite: '官方网站',
    github: 'GitHub',
    latestVersion: '最新版本',
    updateAvailable: '发现新版本',
    noUpdateAvailable: '当前已是最新版本',
    updateUnavailable: '暂时无法获取更新信息',
    openReleasePage: '打开发布页',
    openDownloadLink: '打开下载入口',
    releaseAsset: '安装包',
    publishedAt: '发布时间',
    appIntro: '面向程序员的轻量环境管理工具，用于扫描开发环境、缓存、工具链、PATH 和隔离区。',
    donationNote: '如果这个工具帮你省下了清理环境的时间，可以扫码支持后续开发。',
    scanning: '扫描中',
    developer: '开发环境',
    driveMode: '指定盘符',
    fullMode: '全盘',
    all: '全部',
    existingOnly: '只看存在',
    found: '发现目录',
    computed: '已估算大小',
    cleanable: '可清理',
    review: '需确认',
    speed: '扫描耗时',
    memory: '结果内存',
    selected: '已选',
    search: '搜索名称或路径',
    scanResultList: '扫描结果列表',
    batchDeep: '批量深扫',
    batchQuarantine: '移入隔离区',
    name: '名称',
    path: '路径',
    size: '大小',
    category: '类别',
    risk: '风险',
    status: '状态',
    action: '操作',
    exists: '存在',
    missing: '未发现',
    approx: '估算',
    exact: '精确',
    pendingSize: '待深扫',
    deepScan: '深扫',
    deepScanning: '深扫中',
    quarantineAction: '隔离',
    deleting: '处理中',
    details: '详情',
    children: '子项明细',
    quarantine: '隔离区',
    restore: '恢复',
    permanentlyDelete: '永久删除',
    pathHealth: 'PATH 健康',
    rules: '规则',
    graph: '依赖',
    empty: '正在初始化，若未自动扫描请点击“扫描”。',
    keep: '保留',
    caution: '谨慎',
    runtime: '运行时',
    cache: '缓存',
    config: '配置',
    temp: '临时',
    toolchain: '工具链',
    packageManager: '包管理',
    aiTool: 'AI 工具',
    ide: 'IDE',
    container: '容器',
    mobile: '移动开发',
    buildTool: '构建工具',
    drive: '磁盘',
  }

  const en: typeof zh = {
    title: 'DevEnv Manager',
    subtitle: 'Scan developer environments, see size and risk first, then deep scan or quarantine.',
    scan: 'Scan',
    rescan: 'Rescan',
    export: 'Export',
    settings: 'Settings',
    back: 'Back',
    theme: 'Theme',
    themeStyle: 'Color style',
    themeMist: 'Apple gray-blue',
    themeGraphite: 'Graphite',
    themeAzure: 'Microsoft blue',
    themeGithub: 'GitHub neutral',
    themeNotion: 'Notion ink',
    lightTheme: 'Light',
    darkTheme: 'Dark',
    deepPage: 'Deep scan',
    deepTarget: 'Target',
    deepSummary: 'Summary',
    childRanking: 'Child ranking',
    quickActions: 'Quick actions',
    openFolder: 'Open Folder',
    focusView: 'Focus',
    batchActions: 'Batch actions',
    diagnostics: 'Diagnostics',
    auditLog: 'Audit log',
    environment: 'Environment',
    mainPage: 'Environment scan',
    environmentPage: 'Environment check',
    checkEnvironment: 'Check environment',
    environmentIntro: 'Check installed developer tools and generate confirmable install plans for missing items.',
    quickInstall: 'Quick install',
    installable: 'Installable',
    checkedTools: 'Tool details',
    packageManagerStatus: 'Package manager',
    requiredMissing: 'Required missing',
    optionalMissing: 'Optional missing',
    installed: 'Installed',
    missingTool: 'Missing',
    install: 'Install',
    installing: 'Installing',
    installPlan: 'Install plan',
    environmentSuites: 'Environment suites',
    installSuite: 'Install suite',
    suiteComplete: 'Complete',
    suiteMissing: 'Missing',
    needsAdmin: 'May require administrator privileges',
    coreRequired: 'Required',
    optional: 'Optional',
    noInstallPlan: 'No automatic install plan is available for this platform.',
    noInstallPlanShort: 'No plan',
    installConfirm: 'Confirm install',
    installResult: 'Install result',
    uninstall: 'Uninstall',
    uninstalling: 'Uninstalling',
    uninstallConfirm: 'Confirm uninstall',
    uninstallCommand: 'Uninstall command',
    uninstallSafetyHint: 'The uninstall command above will run only after this second confirmation.',
    uninstallDetailOnly: 'Uninstall can only be started from the tool detail page.',
    noUninstallPlan: 'No automatic uninstall plan is available for this source.',
    cancel: 'Cancel',
    openTerminal: 'Open terminal',
    copyCommand: 'Copy command',
    version: 'Version',
    reinstall: 'Reinstall / switch version',
    openDetails: 'Details',
    toolSource: 'Source',
    installCommand: 'Install command',
    detectedVersion: 'Detected version',
    selectedVersion: 'Select version',
    metadataOverview: 'Metadata',
    sourceDetails: 'Install sources',
    links: 'Links',
    homepage: 'Homepage',
    docs: 'Docs',
    usage: 'Usage',
    keywords: 'Keywords',
    primaryUseCases: 'Use cases',
    relatedTools: 'Related tools',
    notes: 'Notes',
    installNotes: 'Install notes',
    upgradeNotes: 'Upgrade notes',
    knownIssues: 'Known issues',
    verifyCommands: 'Verify commands',
    downloads: 'Downloads',
    publisher: 'Publisher',
    license: 'License',
    qualityScore: 'Quality score',
    confidence: 'Confidence',
    scanStatus: 'Scan status',
    scannedAt: 'Scanned at',
    versions: 'Versions',
    latest: 'Latest',
    prerelease: 'Prerelease',
    releaseDate: 'Release date',
    eolDate: 'EOL',
    changelog: 'Changelog',
    packageId: 'Package ID',
    officialSource: 'Official source',
    directDownload: 'Direct',
    noMetadata: 'No extra metadata yet.',
    commandTemplate: 'Command template',
    lifecycle: 'Lifecycle',
    riskInfo: 'Risk',
    recommendedInstall: 'Recommended',
    recommendedInstallIntro: 'Prioritize required tools and common developer tools with built-in install plans.',
    missingEnvironments: 'Missing environments',
    installedEnvironments: 'Installed environments',
    pendingEnvironments: 'Checking',
    noRecommendedInstall: 'No recommended installs right now.',
    noMissingEnvironments: 'All environments are installed or no missing items were found.',
    noInstalledEnvironments: 'No installed environments were detected yet.',
    environmentChecking: 'Checking environment',
    environmentProgress: 'Checked',
    sizeLoading: 'Loading actual sizes',
    openEnvironmentWindow: 'Open environment check',
    ruleCoverage: 'Rule coverage',
    scanScope: 'Scan scope',
    scanDrive: 'Scan drive',
    pendingScan: 'Needs deep scan',
    refreshSize: 'Refresh size',
    safetyPreview: 'Safety preview',
    estimatedFiles: 'Files',
    estimatedDirs: 'Dirs',
    noAuditLog: 'No audit entries yet.',
    sortBy: 'Sort',
    sortSize: 'Size',
    sortName: 'Name',
    sortRisk: 'Risk',
    sortStatus: 'Status',
    selectCleanable: 'Select cleanable',
    clearSelection: 'Clear selection',
    overview: 'Overview',
    cleanupView: 'Cleanup',
    issuesView: 'Issues',
    toolchainView: 'Toolchains',
    packageView: 'Packages',
    aiView: 'AI tools',
    largeView: 'Large',
    smartDeepScan: 'Smart deep scan',
    actionPlan: 'Action plan',
    noPathIssues: 'No PATH issues found.',
    noPlan: 'No recommendations. The scan result looks clean.',
    about: 'About',
    updateCheck: 'Check Updates',
    donate: 'Donate',
    close: 'Close',
    appVersion: 'Version',
    buildType: 'Build',
    portableBuild: 'Portable / Installer',
    languageSetting: 'Language',
    updateStatus: 'Checks GitHub Releases for the latest installer, then opens the release page or download entry.',
    checkNow: 'Check now',
    checkingUpdate: 'Checking',
    officialWebsite: 'Official website',
    github: 'GitHub',
    latestVersion: 'Latest version',
    updateAvailable: 'Update available',
    noUpdateAvailable: 'You are on the latest version',
    updateUnavailable: 'Unable to fetch update information',
    openReleasePage: 'Open release page',
    openDownloadLink: 'Open download',
    releaseAsset: 'Installer',
    publishedAt: 'Published',
    appIntro: 'A lightweight developer environment manager for scanning dev environments, caches, toolchains, PATH, and quarantine.',
    donationNote: 'If this tool saves you cleanup time, scan to support continued development.',
    scanning: 'Scanning',
    developer: 'Developer',
    driveMode: 'Drive',
    fullMode: 'Computer',
    all: 'All',
    existingOnly: 'Existing only',
    found: 'Found',
    computed: 'Estimated size',
    cleanable: 'Cleanable',
    review: 'Review',
    speed: 'Scan time',
    memory: 'Memory',
    selected: 'Selected',
    search: 'Search name or path',
    scanResultList: 'Scan result list',
    batchDeep: 'Deep selected',
    batchQuarantine: 'Quarantine',
    name: 'Name',
    path: 'Path',
    size: 'Size',
    category: 'Category',
    risk: 'Risk',
    status: 'Status',
    action: 'Action',
    exists: 'Exists',
    missing: 'Missing',
    approx: 'Estimated',
    exact: 'Exact',
    pendingSize: 'Not scanned',
    deepScan: 'Deep',
    deepScanning: 'Scanning',
    quarantineAction: 'Quarantine',
    deleting: 'Working',
    details: 'Details',
    children: 'Children',
    quarantine: 'Quarantine',
    restore: 'Restore',
    permanentlyDelete: 'Delete',
    pathHealth: 'PATH health',
    rules: 'Rules',
    graph: 'Graph',
    empty: 'Initializing. Click Scan if auto scan did not start.',
    keep: 'Keep',
    caution: 'Caution',
    runtime: 'Runtime',
    cache: 'Cache',
    config: 'Config',
    temp: 'Temp',
    toolchain: 'Toolchain',
    packageManager: 'Package manager',
    aiTool: 'AI tool',
    ide: 'IDE',
    container: 'Container',
    mobile: 'Mobile',
    buildTool: 'Build tool',
    drive: 'Drive',
  }

  const names: Record<Locale, Record<string, string>> = {
    'zh-CN': {
      'env.codexHome': 'Codex 主目录',
      'env.codexSkills': 'Codex Skills',
      'env.codexCache': 'Codex 运行时缓存',
      'env.generalCache': '通用缓存',
      'env.cargoHome': 'Cargo 主目录',
      'env.cargoBin': 'Cargo 命令入口',
      'env.rustupHome': 'Rustup 主目录',
      'env.rustToolchains': 'Rust 工具链',
      'env.openclawHome': 'OpenClaw',
      'env.qclawHome': 'QClaw',
      'env.codeiumHome': 'Codeium/Windsurf',
      'env.configHome': '配置目录',
      'env.goPath': 'Go 工作区',
      'env.goCache': 'Go 构建缓存',
      'env.mavenHome': 'Maven 本地仓库',
      'env.gradleHome': 'Gradle 主目录',
      'env.ivyHome': 'Ivy 缓存',
      'env.sbtHome': 'sbt 缓存',
      'env.androidGradle': 'Android/Gradle 缂撳瓨',
      'env.denoHome': 'Deno',
      'env.bunHome': 'Bun',
      'env.pnpmHome': 'pnpm Store',
      'env.npmCache': 'npm 缓存',
      'env.npmGlobal': 'npm 全局目录',
      'env.yarnCache': 'Yarn 缓存',
      'env.pnpmLocal': 'pnpm 本地缓存',
      'env.yarnLocalCache': 'Yarn 本地缓存',
      'env.pipCache': 'pip 缓存',
      'env.pythonPrograms': 'Python 安装',
      'env.poetryCache': 'Poetry 缓存',
      'env.uvCache': 'uv 缓存',
      'env.condaHome': 'Conda',
      'env.dockerHome': 'Docker 配置',
      'env.dockerDesktop': 'Docker Desktop',
      'env.vscodeHome': 'VS Code 配置',
      'env.vscodeRoaming': 'VS Code 用户数据',
      'env.jetbrainsCache': 'JetBrains 数据',
      'env.visualStudioCache': 'Visual Studio 数据',
      'env.nugetCache': 'NuGet 包缓存',
      'env.androidHome': 'Android 配置',
      'env.androidSdk': 'Android SDK',
      'env.flutterHome': 'Flutter 配置',
      'env.openaiLocal': 'OpenAI 本地数据',
      'env.claudeHome': 'Claude 数据',
      'env.claudeRoaming': 'Claude 漫游数据',
      'env.wingetCache': 'winget 缓存',
      'env.temp': '临时目录',
      'env.driveUsers': '用户目录集合',
      'env.programFiles': 'Program Files',
      'env.programFilesX86': 'Program Files (x86)',
      'env.programData': 'ProgramData',
      'env.windowsTemp': 'Windows 临时目录',
      'env.windowsDownloaded': 'Windows 更新缓存',
      'env.vscodeExtensions': 'VS Code 扩展',
      'env.nodeGlobalModules': 'Node 全局模块',
      'env.goModuleCache': 'Go 模块缓存',
      'env.cargoRegistry': 'Cargo Registry',
      'env.cargoGitCache': 'Cargo Git 缓存',
      'env.rustTargetCache': 'Rust 构建产物',
      'env.dockerWslData': 'Docker WSL 数据',
      'env.visualStudioPackages': 'Visual Studio 安装包缓存',
      'env.dotnetTools': '.NET 全局工具',
      'env.denoCache': 'Deno 缓存',
      'env.bunInstallCache': 'Bun 安装缓存',
      'env.androidBuildCache': 'Android 构建缓存',
      'env.nvmHome': 'nvm Node 版本',
      'env.fnmHome': 'fnm Node 版本',
      'env.voltaHome': 'Volta 工具链',
      'env.javaJdks': '本地 JDK 集合',
      'env.sdkmanHome': 'SDKMAN 主目录',
      'env.jbangCache': 'JBang 缓存',
      'env.dotnetHome': '.NET 主目录',
      'env.pipxHome': 'pipx 全局工具',
      'env.pyenvHome': 'pyenv Python 版本',
      'env.virtualenvCache': 'virtualenv 缓存',
      'env.scoopHome': 'Scoop 主目录',
      'env.scoopCache': 'Scoop 下载缓存',
      'env.chocolateyHome': 'Chocolatey 主目录',
      'env.ghConfig': 'GitHub CLI 配置',
      'env.awsConfig': 'AWS 配置',
      'env.azureConfig': 'Azure 配置',
      'env.gcloudConfig': 'Google Cloud 配置',
      'env.terraformCache': 'Terraform 插件缓存',
      'env.kubeConfig': 'Kubernetes 配置',
      'env.minikubeHome': 'Minikube 数据',
      'env.podmanHome': 'Podman 配置',
      'env.cursorUserData': 'Cursor 用户数据',
      'env.windsurfUserData': 'Windsurf 用户数据',
      'env.traeUserData': 'Trae 用户数据',
      'env.composerCache': 'Composer 缓存',
      'env.rubyGemHome': 'RubyGems 主目录',
      'env.stackHome': 'Stack 主目录',
      'env.cabalHome': 'Cabal 主目录',
    },
    'en-US': {},
  }
  names['en-US'] = Object.fromEntries(Object.entries(names['zh-CN']).map(([key]) => [key, key.replace('env.', '')]))

  let locale: Locale = 'zh-CN'
  let report: ScanReport | null = null
  let entries: EnvEntry[] = []
  let drives: string[] = []
  let selectedDrive = ''
  let scanMode: ScanMode = 'drive'
  let loading = false
  let error = ''
  let exportedPath = ''
  let searchQuery = ''
  let selectedIds = new Set<string>()
  let focusView: FocusView = 'overview'
  let sortMode: SortMode = 'sizeDesc'
  let themeMode: ThemeMode = 'azure'
  let pageMode: PageMode = 'main'
  let existingOnly = true
  let lastScanMs = 0
  let deepScanning = new Set<string>()
  let deleting = new Set<string>()
  let childEntries: EnvEntry[] = []
  let insights: ToolInsights | null = null
  let deepEntry: EnvEntry | null = null
  let deepReport: DeepScanReport | null = null
  let quarantineItems: QuarantineItem[] = []
  let auditLog: AuditLogEntry[] = []
  let deletingQuarantine = new Set<string>()
  let refreshingSize = new Set<string>()
  let autoSizing = false
  let tableScrollDrag: ScrollDragState | null = null
  let environmentReport: EnvironmentCheckReport | null = null
  let environmentLoading = false
  let environmentToolDetail: DevToolCheck | null = null
  let environmentToolDetailLoading = false
  let installingTools = new Set<string>()
  let uninstallingTools = new Set<string>()
  let confirmingUninstallToolId: string | null = null
  let installMessage = ''
  let environmentTab: EnvironmentTab = 'recommended'
  let selectedInstallPackages: Record<string, string> = {}
  let focusCounts: Record<FocusView, number> = {
    overview: 0,
    cleanup: 0,
    issues: 0,
    toolchains: 0,
    packages: 0,
    ai: 0,
    large: 0,
  }
  let settingsOpen = false
  let settingsTab: SettingsTab = 'settings'
  let appMetadata: AppMetadata = {
    currentVersion: '0.1.0',
    githubUrl: 'https://github.com/GaryIndex/devenv-manager',
    releasesUrl: 'https://github.com/GaryIndex/devenv-manager/releases',
    officialUrl: 'https://github.com/GaryIndex/devenv-manager',
  }
  const themeModes: ThemeMode[] = ['azure', 'graphite', 'github', 'notion', 'mist', 'dark']
  const themeStorageKey = 'devenv-manager-theme'
  const themeBroadcastName = 'devenv-manager-theme'
  let themeBroadcast: BroadcastChannel | null = null
  let updateInfo: AppUpdateInfo | null = null
  let updateLoading = false
  let updateChecked = false
  const environmentWindowLabel = 'environment-check'
  const fullScanValue = '__full__'
  const isEnvironmentWindow = isEnvironmentView()
  const isEnvironmentToolWindow = isEnvironmentToolView()
  const environmentToolId = currentEnvironmentToolId()
  const recommendedSeedToolIds = new Set(['git', 'github_cli', 'vscode', 'windows_terminal', 'powershell', 'ripgrep', 'fd', 'jq', 'node_lts', 'npm', 'pnpm', 'python_312', 'uv', 'rustup', 'go', 'docker_desktop'])

  $: t = locale === 'zh-CN' ? zh : en
  $: headerTitle = isEnvironmentToolWindow
    ? (environmentToolDetail?.name ?? t.details)
    : isEnvironmentWindow
      ? t.environmentPage
      : pageMode === 'deep'
        ? t.deepPage
        : t.title
  $: headerSubtitle = isEnvironmentToolWindow
    ? t.environmentIntro
    : isEnvironmentWindow
      ? t.environmentIntro
      : pageMode === 'deep' && deepEntry
        ? label(deepEntry.nameKey)
        : t.subtitle
  $: filteredEntries = entries
    .filter((entry) => (existingOnly ? entry.exists : true))
    .filter((entry) => matchesFocus(entry, focusView))
    .filter((entry) => {
      const query = searchQuery.trim().toLowerCase()
      return query.length === 0 || label(entry.nameKey).toLowerCase().includes(query) || entry.path.toLowerCase().includes(query)
    })
    .sort(compareEntries)
  $: existingEntries = entries.filter((entry) => entry.exists)
  $: existingCount = existingEntries.length
  $: computedBytes = entries.reduce((sum, entry) => sum + entry.sizeBytes, 0)
  $: cleanableBytes = entries.filter((entry) => entry.exists && entry.risk === 'cleanable').reduce((sum, entry) => sum + entry.sizeBytes, 0)
  $: reviewCount = entries.filter((entry) => entry.exists && entry.risk === 'caution').length
  $: selectedEntries = entries.filter((entry) => selectedIds.has(entry.id))
  $: brokenPathCount = report?.pathEntries.filter((entry) => !entry.exists).length ?? 0
  $: brokenPathEntries = report?.pathEntries.filter((entry) => !entry.exists).slice(0, 6) ?? []
  $: largestEntries = entries.filter((entry) => entry.exists).slice().sort((a, b) => b.sizeBytes - a.sizeBytes).slice(0, 5)
  $: recommendedEntries = entries
    .filter((entry) => entry.exists && (entry.risk === 'cleanable' || entry.sizeBytes >= 1024 * 1024 * 512))
    .slice()
    .sort((a, b) => b.sizeBytes - a.sizeBytes)
    .slice(0, 5)
  $: pendingSizeEntries = entries.filter((entry) => entry.exists && entry.sizeApproximate && entry.sizeBytes === 0).length
  $: approximateSizeEntries = entries.filter((entry) => entry.exists && entry.sizeApproximate).length
  $: installableToolCount = environmentReport?.tools.filter((tool) => tool.status === 'missing' && tool.installPlan).length ?? 0
  $: missingTools = environmentReport?.tools.filter((tool) => tool.status === 'missing') ?? []
  $: installedTools = environmentReport?.tools.filter((tool) => tool.status === 'installed') ?? []
  $: pendingTools = environmentReport?.tools.filter((tool) => tool.status === 'pending') ?? []
  $: recommendedTools = missingTools.filter((tool) => tool.required || (recommendedSeedToolIds.has(tool.id) && Boolean(tool.installPlan)))
  $: environmentTabTools = environmentTab === 'installed' ? installedTools : environmentTab === 'missing' ? missingTools : recommendedTools
  $: focusCounts = {
    overview: countFocus(entries, existingOnly, 'overview'),
    cleanup: countFocus(entries, existingOnly, 'cleanup'),
    issues: countFocus(entries, existingOnly, 'issues'),
    toolchains: countFocus(entries, existingOnly, 'toolchains'),
    packages: countFocus(entries, existingOnly, 'packages'),
    ai: countFocus(entries, existingOnly, 'ai'),
    large: countFocus(entries, existingOnly, 'large'),
  }

  onMount(() => {
    let stopListening: (() => void) | undefined
    const cleanupThemeSync = initializeThemeSync()
    if (isEnvironmentToolWindow) {
      void initialize()
      return () => {
        stopListening?.()
        cleanupThemeSync()
      }
    }
    if (isEnvironmentWindow) {
      listen<EnvironmentCheckProgress>('environment-check-progress', (event) => {
        environmentReport = event.payload.report
        environmentLoading = !event.payload.completed
      }).then((unlisten) => {
        stopListening = unlisten
        void initialize()
      }).catch((err) => {
        error = String(err)
        void initialize()
      })
      return () => {
        stopListening?.()
        cleanupThemeSync()
      }
    }
    listen<ScanProgress>('scan-progress', (event) => {
      upsertEntry(event.payload.entry)
    }).then((unlisten) => {
      stopListening = unlisten
      void initialize()
    }).catch((err) => {
      error = String(err)
      void initialize()
    })
    return () => {
      stopListening?.()
      cleanupThemeSync()
    }
  })

  async function initialize() {
    try {
      appMetadata = await invoke<AppMetadata>('app_metadata')
      drives = await invoke<string[]>('list_drives')
      selectedDrive = drives[0] ?? ''
      scanMode = selectedDrive ? 'drive' : 'full'
      if (isEnvironmentToolWindow) {
        await loadEnvironmentToolDetail()
      } else if (isEnvironmentWindow) {
        await checkEnvironment()
      } else {
        await scan()
      }
    } catch (err) {
      error = String(err)
    }
  }

  function isThemeMode(value: string): value is ThemeMode {
    return themeModes.includes(value as ThemeMode)
  }

  function initializeThemeSync() {
    try {
      const storedTheme = window.localStorage.getItem(themeStorageKey)
      if (storedTheme && isThemeMode(storedTheme)) {
        themeMode = storedTheme
      }
    } catch {
      // Theme still works for the current window if storage is unavailable.
    }

    if (typeof BroadcastChannel === 'undefined') return () => undefined

    themeBroadcast = new BroadcastChannel(themeBroadcastName)
    themeBroadcast.onmessage = (event) => {
      const nextTheme = String(event.data ?? '')
      if (isThemeMode(nextTheme)) {
        themeMode = nextTheme
      }
    }
    return () => {
      themeBroadcast?.close()
      themeBroadcast = null
    }
  }

  function setThemeMode(nextTheme: ThemeMode) {
    themeMode = nextTheme
    try {
      window.localStorage.setItem(themeStorageKey, nextTheme)
    } catch {
      // Broadcast still updates open windows if storage is unavailable.
    }
    themeBroadcast?.postMessage(nextTheme)
  }

  function toggleThemeMode() {
    setThemeMode(themeMode === 'dark' ? 'azure' : 'dark')
  }

  function changeThemeMode(event: Event) {
    const nextTheme = (event.currentTarget as HTMLSelectElement).value
    if (isThemeMode(nextTheme)) {
      setThemeMode(nextTheme)
    }
  }

  async function scan() {
    loading = true
    error = ''
    entries = []
    selectedIds = new Set()
    childEntries = []
    deepEntry = null
    deepReport = null
    pageMode = 'main'
    const started = performance.now()
    try {
      report = await invoke<ScanReport>('scan_environment', {
        request: { mode: scanMode, drive: scanMode === 'drive' ? selectedDrive : null },
      })
      entries = report.entries
      insights = await invoke<ToolInsights>('tool_insights', { report })
      quarantineItems = await invoke<QuarantineItem[]>('list_quarantine')
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      lastScanMs = report.elapsedMs || Math.round(performance.now() - started)
      void refreshApproximateSizes()
    } catch (err) {
      error = String(err)
    } finally {
      loading = false
    }
  }

  async function refreshApproximateSizes() {
    if (autoSizing) return
    autoSizing = true
    try {
      const targets = entries.filter((entry) => entry.exists && entry.sizeApproximate)
      for (const entry of targets) {
        if (!entries.some((item) => item.id === entry.id && item.exists && item.sizeApproximate)) continue
        await refreshEntrySize(entry)
      }
    } finally {
      autoSizing = false
    }
  }

  async function deepScan(entry: EnvEntry) {
    deepScanning = new Set(deepScanning).add(entry.id)
    deepEntry = entry
    deepReport = null
    childEntries = []
    pageMode = 'deep'
    try {
      const deep = await invoke<DeepScanReport>('deep_scan_entry', { path: entry.path })
      const updated = { ...entry, sizeBytes: deep.estimate.bytes, fileCount: deep.estimate.files, dirCount: deep.estimate.dirs, sizeApproximate: deep.estimate.truncated }
      upsertEntry(updated)
      deepEntry = updated
      deepReport = deep
      childEntries = deep.children
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(deepScanning)
      next.delete(entry.id)
      deepScanning = next
    }
  }

  async function quarantineEntry(entry: EnvEntry) {
    if (!entry.exists || entry.risk !== 'cleanable') return
    deleting = new Set(deleting).add(entry.id)
    try {
      const preview = await invoke<CleanupPreview>('preview_cleanup', { path: entry.path, risk: entry.risk })
      if (!confirmPreview(entry, preview)) return
      await invoke('quarantine_entry', { path: entry.path, risk: entry.risk })
      quarantineItems = await invoke<QuarantineItem[]>('list_quarantine')
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      upsertEntry({ ...entry, exists: false, sizeBytes: 0, fileCount: 0, dirCount: 0, sizeApproximate: false })
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(deleting)
      next.delete(entry.id)
      deleting = next
    }
  }

  async function deepScanSelected() {
    for (const entry of selectedEntries.filter((item) => item.exists)) await deepScan(entry)
  }

  async function smartDeepScan() {
    const targets = filteredEntries.filter((entry) => entry.exists).slice(0, 6)
    for (const entry of targets) await deepScan(entry)
  }

  async function quarantineSelected() {
    const targets = selectedEntries.filter((item) => item.exists && item.risk === 'cleanable')
    if (targets.length === 0) return
    const total = targets.reduce((sum, entry) => sum + entry.sizeBytes, 0)
    const message = locale === 'zh-CN'
      ? `确认将 ${targets.length} 个项目移入隔离区？\n估算大小：${formatBytes(total)}\n隔离后可以在隔离区恢复。`
      : `Move ${targets.length} items to quarantine?\nEstimated size: ${formatBytes(total)}\nYou can restore them from quarantine.`
    if (!window.confirm(message)) return
    for (const entry of targets) await quarantineEntryWithoutPrompt(entry)
  }

  async function quarantineEntryWithoutPrompt(entry: EnvEntry) {
    deleting = new Set(deleting).add(entry.id)
    try {
      await invoke('quarantine_entry', { path: entry.path, risk: entry.risk })
      quarantineItems = await invoke<QuarantineItem[]>('list_quarantine')
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      upsertEntry({ ...entry, exists: false, sizeBytes: 0, fileCount: 0, dirCount: 0, sizeApproximate: false })
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(deleting)
      next.delete(entry.id)
      deleting = next
    }
  }

  async function restoreItem(item: QuarantineItem) {
    try {
      await invoke('restore_quarantine', { id: item.id })
      quarantineItems = await invoke<QuarantineItem[]>('list_quarantine')
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      await scan()
    } catch (err) {
      error = String(err)
    }
  }

  async function deleteQuarantineItem(item: QuarantineItem) {
    const message = locale === 'zh-CN'
      ? `确认永久删除隔离项目？\n${item.originalPath}`
      : `Permanently delete quarantined item?\n${item.originalPath}`
    if (!window.confirm(message)) return
    deletingQuarantine = new Set(deletingQuarantine).add(item.id)
    try {
      await invoke('delete_quarantine', { id: item.id })
      quarantineItems = await invoke<QuarantineItem[]>('list_quarantine')
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(deletingQuarantine)
      next.delete(item.id)
      deletingQuarantine = next
    }
  }

  async function exportReport() {
    if (!report) return
    try {
      exportedPath = await invoke<string>('export_html_report', { report: { ...report, entries, totalSizeBytes: computedBytes }, locale })
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
    } catch (err) {
      error = String(err)
    }
  }

  async function openPath(path: string) {
    try {
      await invoke('open_path', { path })
    } catch (err) {
      error = String(err)
    }
  }

  async function checkForAppUpdate() {
    updateLoading = true
    updateChecked = true
    error = ''
    try {
      updateInfo = await invoke<AppUpdateInfo>('check_app_update')
    } catch (err) {
      updateInfo = {
        currentVersion: appMetadata.currentVersion,
        updateAvailable: false,
        releaseUrl: appMetadata.releasesUrl,
        error: String(err),
      }
    } finally {
      updateLoading = false
    }
  }

  async function openExternalUrl(url?: string) {
    if (!url) return
    try {
      await invoke('open_external_url', { url })
    } catch (err) {
      error = String(err)
    }
  }

  async function refreshEntrySize(entry: EnvEntry) {
    if (!entry.exists) return
    refreshingSize = new Set(refreshingSize).add(entry.id)
    try {
      const estimate = await invoke<SizeEstimate>('scan_entry_size', { path: entry.path })
      upsertEntry({ ...entry, sizeBytes: estimate.bytes, fileCount: estimate.files, dirCount: estimate.dirs, sizeApproximate: estimate.truncated })
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(refreshingSize)
      next.delete(entry.id)
      refreshingSize = next
    }
  }

  async function checkEnvironment() {
    environmentLoading = true
    installMessage = ''
    error = ''
    try {
      environmentReport = await invoke<EnvironmentCheckReport>('start_development_environment_check')
    } catch (err) {
      error = String(err)
      environmentLoading = false
    }
  }

  async function loadEnvironmentToolDetail() {
    if (!environmentToolId) {
      error = locale === 'zh-CN' ? '缺少工具 ID。' : 'Missing tool id.'
      return
    }
    environmentToolDetailLoading = true
    installMessage = ''
    error = ''
    try {
      environmentToolDetail = await invoke<DevToolCheck>('get_environment_tool_detail', { request: { toolId: environmentToolId } })
    } catch (err) {
      error = String(err)
    } finally {
      environmentToolDetailLoading = false
    }
  }

  async function openToolDetail(tool: DevToolCheck) {
    try {
      await invoke('open_environment_tool_detail_window', { request: { toolId: tool.id } })
    } catch (err) {
      error = String(err)
    }
  }

  function selectedPackageId(tool: DevToolCheck) {
    return selectedInstallPackages[tool.id] || tool.versionOptions[0]?.id || ''
  }

  function selectedCommand(tool: DevToolCheck) {
    const optionId = selectedPackageId(tool)
    return tool.versionOptions.find((option) => option.id === optionId)?.command ?? tool.installPlan?.command ?? []
  }

  function selectInstallPackage(toolId: string, event: Event) {
    selectedInstallPackages = {
      ...selectedInstallPackages,
      [toolId]: (event.currentTarget as HTMLSelectElement).value,
    }
  }

  async function installTool(tool: DevToolCheck) {
    if (!tool.installPlan) return
    const packageId = selectedPackageId(tool)
    const command = selectedCommand(tool).join(' ')
    const message = locale === 'zh-CN'
      ? `${tool.installed ? t.reinstall : t.installConfirm}: ${tool.name}\n\n${command}\n\n${tool.installPlan.needsAdmin ? t.needsAdmin : t.coreRequired}`
      : `${tool.installed ? t.reinstall : t.installConfirm}: ${tool.name}\n\n${command}\n\n${tool.installPlan.needsAdmin ? t.needsAdmin : t.coreRequired}`
    if (!window.confirm(message)) return
    installingTools = new Set(installingTools).add(tool.id)
    installMessage = ''
    try {
      const result = await invoke<InstallResult>('install_missing_tool', { request: { toolId: tool.id, packageId } })
      installMessage = `${t.installResult}: ${result.status}${result.exitCode === undefined ? '' : ` (${result.exitCode})`}`
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      if (isEnvironmentToolWindow) {
        await loadEnvironmentToolDetail()
      } else {
        await checkEnvironment()
      }
    } catch (err) {
      error = String(err)
    } finally {
      const next = new Set(installingTools)
      next.delete(tool.id)
      installingTools = next
    }
  }

  async function openInstallTerminal(tool: DevToolCheck) {
    if (!tool.installPlan) return
    try {
      await invoke('open_install_terminal', { request: { toolId: tool.id, packageId: selectedPackageId(tool) } })
    } catch (err) {
      error = String(err)
    }
  }

  function canUninstallFromDetail(tool: DevToolCheck) {
    return isEnvironmentToolWindow && environmentToolDetail?.id === tool.id
  }

  function requestUninstallTool(tool: DevToolCheck) {
    if (!tool.installed || !tool.uninstallPlan) return
    if (!canUninstallFromDetail(tool)) {
      error = t.uninstallDetailOnly
      return
    }
    confirmingUninstallToolId = tool.id
  }

  async function uninstallTool(tool: DevToolCheck) {
    if (!tool.installed || !tool.uninstallPlan) return
    if (!canUninstallFromDetail(tool)) {
      error = t.uninstallDetailOnly
      return
    }
    if (confirmingUninstallToolId !== tool.id) {
      confirmingUninstallToolId = tool.id
      return
    }
    uninstallingTools = new Set(uninstallingTools).add(tool.id)
    installMessage = ''
    error = ''
    try {
      const result = await invoke<InstallResult>('uninstall_installed_tool', { request: { toolId: tool.id } })
      installMessage = `${t.installResult}: ${result.status}${result.exitCode === undefined ? '' : ` (${result.exitCode})`}`
      auditLog = await invoke<AuditLogEntry[]>('list_audit_log')
      if (isEnvironmentToolWindow) {
        await loadEnvironmentToolDetail()
      } else {
        await checkEnvironment()
      }
    } catch (err) {
      error = String(err)
    } finally {
      confirmingUninstallToolId = null
      const next = new Set(uninstallingTools)
      next.delete(tool.id)
      uninstallingTools = next
    }
  }

  async function openUninstallTerminal(tool: DevToolCheck) {
    if (!tool.uninstallPlan) return
    try {
      await invoke('open_uninstall_terminal', { request: { toolId: tool.id } })
    } catch (err) {
      error = String(err)
    }
  }

  async function copyInstallCommand(tool: DevToolCheck) {
    const command = selectedCommand(tool).join(' ')
    if (!command) return
    try {
      await navigator.clipboard.writeText(command)
      installMessage = command
    } catch (err) {
      error = String(err)
    }
  }

  function confirmPreview(entry: EnvEntry, preview: CleanupPreview) {
    if (!preview.allowed) {
      const message = locale === 'zh-CN'
        ? `当前项目不允许隔离：${preview.reason}`
        : `This item is not allowed: ${preview.reason}`
      window.alert(message)
      return false
    }
    const message = locale === 'zh-CN'
      ? `${t.safetyPreview}\n\n${label(entry.nameKey)}\n${preview.path}\n\n${t.size}: ${formatBytes(preview.estimate.bytes)}${preview.estimate.truncated ? ` (${t.approx})` : ` (${t.exact})`}\n${t.estimatedFiles}: ${preview.estimate.files}\n${t.estimatedDirs}: ${preview.estimate.dirs}\n\n确认移入隔离区？`
      : `${t.safetyPreview}\n\n${label(entry.nameKey)}\n${preview.path}\n\n${t.size}: ${formatBytes(preview.estimate.bytes)}${preview.estimate.truncated ? ` (${t.approx})` : ` (${t.exact})`}\n${t.estimatedFiles}: ${preview.estimate.files}\n${t.estimatedDirs}: ${preview.estimate.dirs}\n\nMove to quarantine?`
    return window.confirm(message)
  }

  function upsertEntry(entry: EnvEntry) {
    const index = entries.findIndex((item) => item.id === entry.id)
    entries = index === -1 ? [...entries, entry] : entries.map((item) => (item.id === entry.id ? entry : item))
  }

  function toggleSelected(entry: EnvEntry) {
    const next = new Set(selectedIds)
    next.has(entry.id) ? next.delete(entry.id) : next.add(entry.id)
    selectedIds = next
  }

  function toggleAllVisible() {
    const ids = filteredEntries.map((entry) => entry.id)
    const allSelected = ids.length > 0 && ids.every((id) => selectedIds.has(id))
    const next = new Set(selectedIds)
    for (const id of ids) allSelected ? next.delete(id) : next.add(id)
    selectedIds = next
  }

  function beginTableScrollDrag(event: PointerEvent) {
    if (event.pointerType === 'mouse' && event.button !== 0) return
    if (isInteractiveDragTarget(event.target)) return
    const scroller = event.currentTarget as HTMLDivElement
    if (scroller.scrollWidth <= scroller.clientWidth && scroller.scrollHeight <= scroller.clientHeight) return
    tableScrollDrag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      scrollLeft: scroller.scrollLeft,
      scrollTop: scroller.scrollTop,
      active: false,
    }
    scroller.setPointerCapture(event.pointerId)
  }

  function moveTableScrollDrag(event: PointerEvent) {
    const state = tableScrollDrag
    if (!state || state.pointerId !== event.pointerId) return
    const scroller = event.currentTarget as HTMLDivElement
    const deltaX = event.clientX - state.startX
    const deltaY = event.clientY - state.startY
    const active = state.active || Math.abs(deltaX) > 3 || Math.abs(deltaY) > 3
    tableScrollDrag = { ...state, active }
    if (!active) return
    scroller.scrollLeft = state.scrollLeft - deltaX
    scroller.scrollTop = state.scrollTop - deltaY
    event.preventDefault()
  }

  function endTableScrollDrag(event: PointerEvent) {
    const state = tableScrollDrag
    if (!state || state.pointerId !== event.pointerId) return
    const scroller = event.currentTarget as HTMLDivElement
    if (scroller.hasPointerCapture(event.pointerId)) scroller.releasePointerCapture(event.pointerId)
    tableScrollDrag = null
  }

  function scrollTableWithKeyboard(event: KeyboardEvent) {
    const scroller = event.currentTarget as HTMLDivElement
    const distance = event.shiftKey ? 260 : 80
    const pageDistance = Math.max(scroller.clientHeight - 48, distance)
    let left = 0
    let top = 0
    if (event.key === 'ArrowLeft') left = -distance
    else if (event.key === 'ArrowRight') left = distance
    else if (event.key === 'ArrowUp') top = -distance
    else if (event.key === 'ArrowDown') top = distance
    else if (event.key === 'PageUp') top = -pageDistance
    else if (event.key === 'PageDown') top = pageDistance
    else if (event.key === 'Home') {
      left = event.ctrlKey ? -scroller.scrollWidth : 0
      top = -scroller.scrollHeight
    } else if (event.key === 'End') {
      left = event.ctrlKey ? scroller.scrollWidth : 0
      top = scroller.scrollHeight
    } else {
      return
    }
    scroller.scrollBy({ left, top })
    event.preventDefault()
  }

  function isInteractiveDragTarget(target: EventTarget | null) {
    return target instanceof HTMLElement && Boolean(target.closest('button, input, select, textarea, a, label, summary'))
  }

  function selectCleanableInView() {
    selectedIds = new Set(filteredEntries.filter((entry) => entry.exists && entry.risk === 'cleanable').map((entry) => entry.id))
  }

  function clearSelection() {
    selectedIds = new Set()
  }

  function label(key: string) {
    if (key.startsWith('literal:')) return key.slice('literal:'.length)
    if (key.startsWith('env.driveRoot:')) return locale === 'zh-CN' ? `磁盘根目录 ${key.split(':').slice(1).join(':')}` : `Drive root ${key.split(':').slice(1).join(':')}`
    return names[locale][key] ?? key
  }

  function formatBytes(bytes: number) {
    if (bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let value = bytes
    let index = 0
    while (value >= 1024 && index < units.length - 1) {
      value /= 1024
      index += 1
    }
    return index === 0 ? `${bytes} ${units[index]}` : `${value.toFixed(1)} ${units[index]}`
  }

  function sizeText(entry: EnvEntry) {
    if (entry.sizeApproximate && entry.sizeBytes === 0) return t.pendingSize
    return `${entry.sizeApproximate ? '~' : ''}${formatBytes(entry.sizeBytes)}`
  }

  function matchesFocus(entry: EnvEntry, view: FocusView) {
    if (view === 'cleanup') return entry.exists && entry.risk === 'cleanable'
    if (view === 'issues') return !entry.exists || entry.risk === 'caution'
    if (view === 'toolchains') return entry.category === 'toolchain' || entry.category === 'runtime' || entry.category === 'buildTool'
    if (view === 'packages') return entry.category === 'packageManager' || entry.category === 'cache'
    if (view === 'ai') return entry.category === 'aiTool'
    if (view === 'large') return entry.exists && entry.sizeBytes >= 1024 * 1024 * 256
    return true
  }

  function compareEntries(a: EnvEntry, b: EnvEntry) {
    const existsSort = Number(b.exists) - Number(a.exists)
    if (existsSort !== 0) return existsSort
    if (sortMode === 'nameAsc') return label(a.nameKey).localeCompare(label(b.nameKey))
    if (sortMode === 'riskDesc') return riskScore(b.risk) - riskScore(a.risk) || b.sizeBytes - a.sizeBytes
    if (sortMode === 'status') return Number(b.exists) - Number(a.exists) || riskScore(b.risk) - riskScore(a.risk)
    return b.sizeBytes - a.sizeBytes
  }

  function riskScore(risk: Risk) {
    if (risk === 'cleanable') return 3
    if (risk === 'caution') return 2
    return 1
  }

  function countFocus(sourceEntries: EnvEntry[], showExistingOnly: boolean, view: FocusView) {
    return sourceEntries.filter((entry) => (showExistingOnly ? entry.exists : true)).filter((entry) => matchesFocus(entry, view)).length
  }

  function changeScanTarget(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value
    if (value === fullScanValue || !value) {
      scanMode = 'full'
      return
    }
    selectedDrive = value
    scanMode = 'drive'
  }

  function isEnvironmentView() {
    if (typeof window === 'undefined') return false
    try {
      if (getCurrentWebviewWindow().label === environmentWindowLabel) return true
    } catch {
      // Browser preview does not expose the Tauri window label.
    }
    return new URLSearchParams(window.location.search).get('view') === 'environment'
  }

  function isEnvironmentToolView() {
    if (typeof window === 'undefined') return false
    try {
      if (getCurrentWebviewWindow().label.startsWith('environment-tool-')) return true
    } catch {
      // Browser preview does not expose the Tauri window label.
    }
    return new URLSearchParams(window.location.search).get('view') === 'environment-tool'
  }

  function currentEnvironmentToolId() {
    if (typeof window === 'undefined') return ''
    try {
      const label = getCurrentWebviewWindow().label
      if (label.startsWith('environment-tool-')) return label.slice('environment-tool-'.length)
    } catch {
      // Browser preview does not expose the Tauri window label.
    }
    return new URLSearchParams(window.location.search).get('toolId') ?? ''
  }

  function toolStatusText(tool: DevToolCheck) {
    if (tool.status === 'pending') return t.environmentChecking
    if (tool.installed) return t.installed
    return t.missingTool
  }

  function toolBodyText(tool: DevToolCheck) {
    return tool.detail?.descriptions.long || tool.description || tool.detail?.descriptions.short || tool.summary || tool.installPlan?.notes || t.noInstallPlan
  }

  function linkEntries(links?: ToolLinks) {
    if (!links) return []
    return (['homepage', 'download', 'releases', 'docs'] as const)
      .map((kind) => ({ kind, url: links[kind] }))
      .filter((entry): entry is { kind: keyof ToolLinks; url: string } => Boolean(entry.url))
  }

  function linkLabel(kind: keyof ToolLinks) {
    if (kind === 'homepage') return t.homepage
    if (kind === 'download') return t.downloads
    if (kind === 'releases') return t.changelog
    return t.docs
  }

  function commandText(command: string[]) {
    return command.join(' ')
  }

  function verifyText(command: ToolVerifyCommand) {
    return [command.command, ...command.args].join(' ')
  }

  function sourceDescription(source: ToolSourceDetail) {
    return source.descriptions.long || source.descriptions.short || source.package.description || ''
  }

  function metadataDate(value?: string) {
    if (!value) return '-'
    return value.replace('T', ' ').replace(/\.\d+Z$/, 'Z')
  }

  function versionMeta(version: ToolVersionDetail) {
    const parts = [version.channel, version.latest ? t.latest : '', version.prerelease ? t.prerelease : '', version.lts ? 'LTS' : ''].filter(Boolean)
    return parts.join(' / ') || '-'
  }

  function themeModeLabel(mode: ThemeMode) {
    if (mode === 'azure') return t.themeAzure
    if (mode === 'graphite') return t.themeGraphite
    if (mode === 'github') return t.themeGithub
    if (mode === 'notion') return t.themeNotion
    if (mode === 'dark') return t.darkTheme
    return t.themeMist
  }

  function hasNotes(notes?: ToolNotesMetadata) {
    return Boolean(notes && (notes.install.length || notes.upgrade.length || notes.knownIssues.length))
  }

  function detailTags(detail: ToolDetailMetadata) {
    return [...new Set([...detail.tags, ...detail.aliases, ...detail.usage.primaryUseCases, ...detail.usage.keywords])].filter(Boolean).slice(0, 18)
  }

  function openSettings(tab: SettingsTab) {
    settingsTab = tab
    settingsOpen = true
    if (tab === 'updates' && !updateChecked) void checkForAppUpdate()
  }

  async function openEnvironmentWindow() {
    try {
      await invoke('open_environment_window')
    } catch (err) {
      error = String(err)
    }
  }

  function stopModalEvent(event: Event) {
    event.stopPropagation()
  }
</script>

<main class={`theme-${themeMode}${isEnvironmentToolWindow ? ' no-topbar' : ''}`}>
  {#if !isEnvironmentToolWindow}
    <header class="topbar">
      <div>
        <h1>{headerTitle}</h1>
        <p>{headerSubtitle}</p>
      </div>
      {#if !isEnvironmentWindow}
        <div class="actions">
          <select bind:value={locale} aria-label="Language">
            <option value="zh-CN">中文</option>
            <option value="en-US">English</option>
          </select>
          <button class="secondary icon-only" title={t.theme} aria-label={t.theme} onclick={toggleThemeMode}>
            {#if themeMode === 'dark'}<Sun size={16} />{:else}<Moon size={16} />{/if}
          </button>
          {#if pageMode !== 'main'}
            <button class="secondary" onclick={() => (pageMode = 'main')}>{t.back}</button>
          {/if}
          <button class="secondary icon-text" onclick={openEnvironmentWindow}><LaptopMinimalCheck size={16} />{t.environmentPage}</button>
          <button class="secondary icon-text" onclick={scan} disabled={loading}><RefreshCw size={16} />{loading ? t.scanning : report ? t.rescan : t.scan}</button>
          <button class="secondary icon-text" onclick={exportReport} disabled={!report}><Download size={16} />{t.export}</button>
          <button class="secondary icon-text" onclick={() => openSettings('settings')}><Settings size={16} />{t.settings}</button>
        </div>
      {/if}
    </header>
  {/if}

  {#if isEnvironmentToolWindow}
    <section class="environment-page tool-detail-page">
      {#if error}<p class="message error">{error}</p>{/if}
      {#if environmentToolDetail}
        <section class="tool-detail-layout">
          <div class="tool-detail-summary">
            <div>
              <span class={`status-pill ${environmentToolDetail.installed ? 'installed' : environmentToolDetail.installPlan ? 'missing' : 'unavailable'}`}>{toolStatusText(environmentToolDetail)}</span>
              <h2>{environmentToolDetail.name}</h2>
              <p>{toolBodyText(environmentToolDetail)}</p>
            </div>
            <dl class="tool-detail-facts">
              <div><dt>{t.category}</dt><dd>{environmentToolDetail.category}</dd></div>
              <div><dt>{t.status}</dt><dd>{toolStatusText(environmentToolDetail)}</dd></div>
              <div><dt>{t.detectedVersion}</dt><dd>{environmentToolDetail.version ?? '-'}</dd></div>
              <div><dt>{t.toolSource}</dt><dd>{environmentToolDetail.installPlan ? `${environmentToolDetail.installPlan.packageManager} / ${environmentToolDetail.installPlan.sourceId}` : environmentToolDetail.source}</dd></div>
            </dl>
          </div>

          <section class="install-detail-panel">
            <div class="section-head">
              <div>
                <h2>{environmentToolDetail.installed ? t.reinstall : t.installPlan}</h2>
                <p>{environmentToolDetail.installPlan?.notes ?? t.noInstallPlan}</p>
              </div>
              {#if environmentToolDetail.installPlan?.needsAdmin}
                <span class="status-pill unavailable">{t.needsAdmin}</span>
              {/if}
            </div>
            {#if environmentToolDetail.installPlan}
              <label class="version-picker">
                <span>{t.selectedVersion}</span>
                <select value={selectedPackageId(environmentToolDetail)} onchange={(event) => selectInstallPackage(environmentToolDetail!.id, event)}>
                  {#each environmentToolDetail.versionOptions as option}
                    <option value={option.id}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <div class="command-block">
                <span>{t.installCommand}</span>
                <code>{selectedCommand(environmentToolDetail).join(' ')}</code>
              </div>
              <div class="install-actions">
                <button class="secondary icon-text" onclick={() => copyInstallCommand(environmentToolDetail!)}><Copy size={14} />{t.copyCommand}</button>
                <button class="secondary icon-text" onclick={() => openInstallTerminal(environmentToolDetail!)}><Terminal size={14} />{t.openTerminal}</button>
                <button class="icon-text" onclick={() => installTool(environmentToolDetail!)} disabled={installingTools.has(environmentToolDetail.id)}><Download size={14} />{installingTools.has(environmentToolDetail.id) ? t.installing : environmentToolDetail.installed ? t.reinstall : t.install}</button>
              </div>
            {/if}
            {#if environmentToolDetail.installed}
              {#if environmentToolDetail.uninstallPlan}
                <div class="command-block">
                  <span>{t.uninstallCommand}</span>
                  <code>{environmentToolDetail.uninstallPlan.command.join(' ')}</code>
                </div>
                <div class="install-actions">
                  <button class="secondary icon-text" onclick={() => openUninstallTerminal(environmentToolDetail!)}><Terminal size={14} />{t.openTerminal}</button>
                  {#if confirmingUninstallToolId === environmentToolDetail.id}
                    <button class="secondary icon-text" onclick={() => (confirmingUninstallToolId = null)} disabled={uninstallingTools.has(environmentToolDetail.id)}>{t.cancel}</button>
                    <button class="danger-button icon-text" onclick={() => uninstallTool(environmentToolDetail!)} disabled={uninstallingTools.has(environmentToolDetail.id)}><Trash2 size={14} />{uninstallingTools.has(environmentToolDetail.id) ? t.uninstalling : t.uninstallConfirm}</button>
                  {:else}
                    <button class="danger-button icon-text" onclick={() => requestUninstallTool(environmentToolDetail!)} disabled={uninstallingTools.has(environmentToolDetail.id)}><Trash2 size={14} />{t.uninstall}</button>
                  {/if}
                </div>
                {#if confirmingUninstallToolId === environmentToolDetail.id}
                  <p class="uninstall-warning">{t.uninstallSafetyHint}</p>
                {/if}
              {:else}
                <p class="empty">{t.noUninstallPlan}</p>
              {/if}
            {:else if !environmentToolDetail.installPlan}
              <p class="empty">{t.noInstallPlan}</p>
            {/if}
          </section>
          {#if environmentToolDetail.detail}
            {@const detail = environmentToolDetail.detail}
            <section class="metadata-detail-grid">
              <section class="metadata-panel metadata-wide">
                <div class="section-head">
                  <div>
                    <h2>{t.metadataOverview}</h2>
                    <p>{detail.descriptions.long || detail.descriptions.short || environmentToolDetail.summary || t.noMetadata}</p>
                  </div>
                  <span class="status-pill installed">{detail.quality.score}</span>
                </div>
                {#if linkEntries(detail.links).length}
                  <div class="link-row">
                    {#each linkEntries(detail.links) as link}
                      <button class="secondary link-button" onclick={() => openExternalUrl(link.url)}>{linkLabel(link.kind)}</button>
                    {/each}
                  </div>
                {/if}
                {#if detailTags(detail).length}
                  <div class="chip-row">
                    {#each detailTags(detail) as tag}
                      <span class="metadata-chip">{tag}</span>
                    {/each}
                  </div>
                {/if}
              </section>

              <section class="metadata-panel">
                <h2>{t.usage}</h2>
                <dl class="metadata-facts">
                  <div><dt>{t.primaryUseCases}</dt><dd>{detail.usage.primaryUseCases.join(' / ') || '-'}</dd></div>
                  <div><dt>{t.relatedTools}</dt><dd>{detail.usage.relatedTools.join(' / ') || '-'}</dd></div>
                  <div><dt>{t.lifecycle}</dt><dd>{detail.lifecycle.status ?? '-'}</dd></div>
                  <div><dt>{t.riskInfo}</dt><dd>{detail.risk.requiresAdmin ? t.needsAdmin : '-'}</dd></div>
                </dl>
              </section>

              <section class="metadata-panel">
                <h2>{t.qualityScore}</h2>
                <dl class="metadata-facts">
                  <div><dt>{t.qualityScore}</dt><dd>{detail.quality.score}</dd></div>
                  <div><dt>{t.confidence}</dt><dd>{detail.quality.confidence || '-'}</dd></div>
                  <div><dt>{t.scannedAt}</dt><dd>{metadataDate(detail.quality.lastSuccessfulScanAt)}</dd></div>
                  <div><dt>{t.officialSource}</dt><dd>{detail.quality.official ? t.exists : '-'}</dd></div>
                </dl>
              </section>

              {#if detail.verifyCommands.length}
                <section class="metadata-panel">
                  <h2>{t.verifyCommands}</h2>
                  <div class="command-list">
                    {#each detail.verifyCommands as command}
                      <code>{verifyText(command)}</code>
                    {/each}
                  </div>
                </section>
              {/if}

              {#if hasNotes(detail.notes)}
                <section class="metadata-panel">
                  <h2>{t.notes}</h2>
                  {#if detail.notes.install.length}
                    <strong>{t.installNotes}</strong>
                    <ul class="compact-list">{#each detail.notes.install as item}<li>{item}</li>{/each}</ul>
                  {/if}
                  {#if detail.notes.upgrade.length}
                    <strong>{t.upgradeNotes}</strong>
                    <ul class="compact-list">{#each detail.notes.upgrade as item}<li>{item}</li>{/each}</ul>
                  {/if}
                  {#if detail.notes.knownIssues.length}
                    <strong>{t.knownIssues}</strong>
                    <ul class="compact-list">{#each detail.notes.knownIssues as item}<li>{item}</li>{/each}</ul>
                  {/if}
                </section>
              {/if}

              <section class="metadata-panel metadata-wide">
                <div class="section-head">
                  <div>
                    <h2>{t.sourceDetails}</h2>
                    <p>{environmentToolDetail.installPlan ? `${environmentToolDetail.installPlan.packageManager} / ${environmentToolDetail.installPlan.sourceQuality}` : t.noInstallPlan}</p>
                  </div>
                </div>
                {#if detail.sources.length}
                  <div class="source-detail-list">
                    {#each detail.sources as source, index}
                      <details class="source-detail" open={source.id === environmentToolDetail.installPlan?.sourceId || index === 0}>
                        <summary>
                          <span>
                            <strong>{source.manager}</strong>
                            <small>{source.packageId ?? source.id}</small>
                          </span>
                          <span class={`status-pill ${source.quality.score >= 75 ? 'installed' : source.quality.score >= 45 ? 'missing' : 'unavailable'}`}>{source.quality.score}</span>
                        </summary>
                        <div class="source-detail-body">
                          <dl class="metadata-facts">
                            <div><dt>{t.packageId}</dt><dd>{source.packageId ?? '-'}</dd></div>
                            <div><dt>{t.publisher}</dt><dd>{source.package.publisher ?? source.package.author ?? '-'}</dd></div>
                            <div><dt>{t.license}</dt><dd>{source.package.license ?? '-'}</dd></div>
                            <div><dt>{t.scanStatus}</dt><dd>{source.scan.status}</dd></div>
                            <div><dt>{t.scannedAt}</dt><dd>{metadataDate(source.scan.scannedAt)}</dd></div>
                            <div><dt>{t.officialSource}</dt><dd>{source.official ? t.exists : '-'}</dd></div>
                          </dl>
                          {#if sourceDescription(source)}
                            <p>{sourceDescription(source)}</p>
                          {/if}
                          {#if linkEntries(source.links).length}
                            <div class="link-row">
                              {#each linkEntries(source.links) as link}
                                <button class="secondary link-button" onclick={() => openExternalUrl(link.url)}>{linkLabel(link.kind)}</button>
                              {/each}
                            </div>
                          {/if}
                          {#if source.commands.length}
                            <div class="metadata-subsection">
                              <strong>{t.installCommand}</strong>
                              <div class="command-list">
                                {#each source.commands as command}
                                  <code>{command.action}: {commandText(command.command)}</code>
                                {/each}
                              </div>
                            </div>
                          {/if}
                          {#if source.versions.length}
                            <div class="metadata-subsection">
                              <strong>{t.versions}</strong>
                              <div class="version-row">
                                {#each source.versions.slice(0, 18) as version}
                                  <span class="metadata-chip">{version.version} · {versionMeta(version)}</span>
                                {/each}
                              </div>
                            </div>
                          {/if}
                          {#if source.downloads.length}
                            <div class="metadata-subsection">
                              <strong>{t.downloads}</strong>
                              <div class="download-list">
                                {#each source.downloads as download}
                                  <div>
                                    <span>{download.kind} {download.version ? ` / ${download.version}` : ''}</span>
                                    <small>{download.urlType}{download.direct ? ` / ${t.directDownload}` : ''}{download.sizeBytes ? ` / ${formatBytes(download.sizeBytes)}` : ''}</small>
                                    <button class="secondary link-button" onclick={() => openExternalUrl(download.url)} disabled={!download.url}>{t.openDownloadLink}</button>
                                  </div>
                                {/each}
                              </div>
                            </div>
                          {/if}
                          {#if source.verifyCommands.length}
                            <div class="metadata-subsection">
                              <strong>{t.verifyCommands}</strong>
                              <div class="command-list">
                                {#each source.verifyCommands as command}
                                  <code>{verifyText(command)}</code>
                                {/each}
                              </div>
                            </div>
                          {/if}
                        </div>
                      </details>
                    {/each}
                  </div>
                {:else}
                  <p class="empty">{t.noMetadata}</p>
                {/if}
              </section>
            </section>
          {:else}
            <p class="empty">{t.noMetadata}</p>
          {/if}
          {#if installMessage}<p class="message">{installMessage}</p>{/if}
        </section>
      {:else}
        <p class="empty">{environmentToolDetailLoading ? t.environmentChecking : t.environmentIntro}</p>
      {/if}
    </section>
  {:else if isEnvironmentWindow}
    <section class="environment-page">
      <section class="environment-panel">
        {#if error}<p class="message error">{error}</p>{/if}
        {#if environmentReport}
          <div class="environment-metrics metrics-stacked">
            <div class="metric-row primary-row">
              <div><span>{t.packageManagerStatus}</span><strong>{environmentReport.packageManagerChecked ? (environmentReport.packageManagerAvailable ? environmentReport.packageManagerVersion ?? 'winget' : t.missingTool) : t.environmentChecking}</strong></div>
            </div>
            <div class="metric-row">
              <div><span>{t.environmentProgress}</span><strong>{environmentReport.checkedTools}/{environmentReport.totalTools}</strong></div>
              <div><span>{t.requiredMissing}</span><strong>{environmentReport.requiredMissing}</strong></div>
              <div><span>{t.optionalMissing}</span><strong>{environmentReport.optionalMissing}</strong></div>
              <div><span>{t.pendingEnvironments}</span><strong>{environmentReport.pendingTools}</strong></div>
            </div>
            <div class="metric-row installable-row">
              <div><span>{t.installable}</span><strong>{installableToolCount}</strong></div>
            </div>
          </div>
          {#if installMessage}<p class="message">{installMessage}</p>{/if}

          <section class="environment-tools-panel">
            <div class="environment-tabs" role="tablist" aria-label={t.environmentPage}>
              <button class:active={environmentTab === 'recommended'} aria-pressed={environmentTab === 'recommended'} onclick={() => (environmentTab = 'recommended')}>
                <ShieldCheck size={15} />
                <span>{t.recommendedInstall}</span>
                <strong>{recommendedTools.length}</strong>
              </button>
              <button class:active={environmentTab === 'missing'} aria-pressed={environmentTab === 'missing'} onclick={() => (environmentTab = 'missing')}>
                <PackageSearch size={15} />
                <span>{t.missingEnvironments}</span>
                <strong>{missingTools.length}</strong>
              </button>
              <button class:active={environmentTab === 'installed'} aria-pressed={environmentTab === 'installed'} onclick={() => (environmentTab = 'installed')}>
                <LaptopMinimalCheck size={15} />
                <span>{t.installedEnvironments}</span>
                <strong>{installedTools.length}</strong>
              </button>
            </div>

            {#if pendingTools.length > 0}
              <div class="pending-tool-grid" aria-live="polite">
                {#each pendingTools.slice(0, 8) as tool}
                  <article class="pending-tool-card">
                    <span></span>
                    <strong>{tool.name}</strong>
                    <small>{t.environmentChecking}</small>
                  </article>
                {/each}
              </div>
            {/if}

            {#if environmentTab === 'recommended'}
              <p class="environment-tab-note">{t.recommendedInstallIntro}</p>
            {/if}

            {#if environmentTabTools.length === 0}
              <p class="empty">{environmentTab === 'recommended' ? t.noRecommendedInstall : environmentTab === 'missing' ? t.noMissingEnvironments : t.noInstalledEnvironments}</p>
            {:else if environmentTab === 'installed'}
              <div class="installed-tool-grid">
                {#each installedTools as tool}
                  <article class="installed-tool-card">
                    <button class="tool-card-button" onclick={() => openToolDetail(tool)}>
                      <span>
                        <strong>{tool.name}</strong>
                        <small>{tool.version ?? tool.source}</small>
                      </span>
                      <span class="status-pill installed">{t.installed}</span>
                    </button>
                  </article>
                {/each}
              </div>
            {:else}
              <div class="missing-tool-grid">
                {#each environmentTabTools as tool}
                  <article class="install-tool-card">
                    <button class="install-tool-summary" onclick={() => openToolDetail(tool)}>
                      <span>
                        <strong>{tool.name}</strong>
                        <small>{tool.required ? t.coreRequired : t.optional} / {tool.category}</small>
                      </span>
                      <span class={`status-pill card-corner-pill ${tool.installPlan ? 'missing' : 'unavailable'}`}>{tool.installPlan ? t.installable : t.noInstallPlanShort}</span>
                    </button>
                  </article>
                {/each}
              </div>
            {/if}
          </section>
        {:else}
          <p class="empty">{environmentLoading ? t.scanning : t.environmentIntro}</p>
        {/if}
      </section>
    </section>
  {:else if pageMode === 'main'}
  <section class="summary-strip">
    <div class="summary-filter-card">
      <HardDrive size={16} />
      <span>{t.scanScope}</span>
      <div class="summary-controls">
        <select class="summary-select" value={scanMode === 'full' ? fullScanValue : selectedDrive} onchange={changeScanTarget} aria-label={t.scanScope}>
          {#each drives as drive}<option value={drive}>{drive}</option>{/each}
          <option value={fullScanValue}>{t.fullMode}</option>
        </select>
      </div>
    </div>
    <div><Database size={16} /><span>{t.computed}</span><strong>{formatBytes(computedBytes)}</strong></div>
    <div><Trash2 size={16} /><span>{t.cleanable}</span><strong>{formatBytes(cleanableBytes)}</strong></div>
    <div><ShieldCheck size={16} /><span>{t.review}</span><strong>{reviewCount}</strong></div>
    <div><Gauge size={16} /><span>{t.speed}</span><strong>{lastScanMs} ms</strong></div>
    <div><ClipboardList size={16} /><span>{autoSizing ? t.sizeLoading : t.pendingScan}</span><strong>{approximateSizeEntries}</strong></div>
  </section>

  <section class="workspace">
    <aside class="nav-panel">
      <details class="collapse-section" open>
        <summary><ChevronDown size={15} /><span>{t.focusView}</span></summary>
        <div class="focus-group">
          <button class:active={focusView === 'overview'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'overview')}><Gauge size={15} /><span>{t.overview}</span><strong>{focusCounts.overview}</strong></button>
          <button class:active={focusView === 'cleanup'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'cleanup')}><Trash2 size={15} /><span>{t.cleanupView}</span><strong>{focusCounts.cleanup}</strong></button>
          <button class:active={focusView === 'issues'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'issues')}><Route size={15} /><span>{t.issuesView}</span><strong>{focusCounts.issues}</strong></button>
          <button class:active={focusView === 'toolchains'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'toolchains')}><Wrench size={15} /><span>{t.toolchainView}</span><strong>{focusCounts.toolchains}</strong></button>
          <button class:active={focusView === 'packages'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'packages')}><PackageSearch size={15} /><span>{t.packageView}</span><strong>{focusCounts.packages}</strong></button>
          <button class:active={focusView === 'ai'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'ai')}><Bot size={15} /><span>{t.aiView}</span><strong>{focusCounts.ai}</strong></button>
          <button class:active={focusView === 'large'} class:loading-focus={loading || autoSizing} onclick={() => (focusView = 'large')}><Database size={15} /><span>{t.largeView}</span><strong>{focusCounts.large}</strong></button>
        </div>
      </details>
      <div class="nav-meta">
        <div><span>{t.pathHealth}</span><strong>{brokenPathCount}</strong></div>
        <div><span>{t.rules}</span><strong>{insights?.configRuleCount ?? 0}</strong></div>
        <div><span>{t.graph}</span><strong>{insights?.dependencyEdgeCount ?? 0}</strong></div>
        <div><span>{t.quarantine}</span><strong>{quarantineItems.length}</strong></div>
      </div>
    </aside>

    <section class="list-panel">
      <div class="panel-head">
        <div>
          <h2>{t.overview}</h2>
          <p>{filteredEntries.length} / {entries.length}</p>
        </div>
        <div class="table-tools">
          <input class="search" bind:value={searchQuery} placeholder={t.search} aria-label={t.search} />
          <label class="inline-check"><input type="checkbox" bind:checked={existingOnly} /> {t.existingOnly}</label>
          <label class="sort-control"><ArrowUpDown size={15} /><span>{t.sortBy}</span>
            <select bind:value={sortMode} aria-label={t.sortBy}>
              <option value="sizeDesc">{t.sortSize}</option>
              <option value="nameAsc">{t.sortName}</option>
              <option value="riskDesc">{t.sortRisk}</option>
              <option value="status">{t.sortStatus}</option>
            </select>
          </label>
        </div>
      </div>
      <details class="batch-panel">
        <summary><ChevronDown size={15} /><span>{t.batchActions}</span><strong>{selectedEntries.length}</strong></summary>
        <div class="batch-actions">
          <button class="secondary icon-text" onclick={smartDeepScan} disabled={filteredEntries.length === 0}><FolderSearch size={15} />{t.smartDeepScan}</button>
          <button class="secondary icon-text" onclick={deepScanSelected} disabled={selectedEntries.length === 0}><ScanSearch size={15} />{t.batchDeep}</button>
          <button class="secondary icon-text" onclick={selectCleanableInView} disabled={filteredEntries.length === 0}><ShieldCheck size={15} />{t.selectCleanable}</button>
          <button class="secondary" onclick={clearSelection} disabled={selectedEntries.length === 0}>{t.clearSelection}</button>
          <button class="danger-button icon-text" onclick={quarantineSelected} disabled={!selectedEntries.some((entry) => entry.exists && entry.risk === 'cleanable')}><ShieldCheck size={15} />{t.batchQuarantine}</button>
        </div>
      </details>
      {#if error}<p class="message error">{error}</p>{/if}
      {#if exportedPath}<p class="message">{exportedPath}</p>{/if}
      <details class="batch-panel">
        <summary><ChevronDown size={15} /><span>{t.quarantine}</span><strong>{quarantineItems.length}</strong></summary>
        <div class="quarantine-list">
          {#each quarantineItems.slice(0, 8) as item}
            <div class="quarantine-row">
              <code>{item.originalPath}</code>
              <div class="quarantine-actions">
                <button class="link-button icon-text" onclick={() => restoreItem(item)}><RotateCcw size={14} />{t.restore}</button>
                <button class="danger-button icon-text" onclick={() => deleteQuarantineItem(item)} disabled={deletingQuarantine.has(item.id)}><Trash2 size={14} />{deletingQuarantine.has(item.id) ? t.deleting : t.permanentlyDelete}</button>
              </div>
            </div>
          {/each}
        </div>
      </details>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class:table-dragging={Boolean(tableScrollDrag?.active)}
        class="table-wrap"
        role="region"
        aria-label={t.scanResultList}
        tabindex="0"
        onpointerdown={beginTableScrollDrag}
        onpointermove={moveTableScrollDrag}
        onpointerup={endTableScrollDrag}
        onpointercancel={endTableScrollDrag}
        onkeydown={scrollTableWithKeyboard}
      >
        {#if filteredEntries.length === 0}
          <p class="empty">{t.empty}</p>
        {:else}
          <table>
            <thead>
              <tr>
                <th class="sticky-check"><input type="checkbox" aria-label={t.selected} onchange={toggleAllVisible} checked={filteredEntries.length > 0 && filteredEntries.every((entry) => selectedIds.has(entry.id))} /></th>
                <th class="sticky-name">{t.name}</th>
                <th>{t.size}</th>
                <th>{t.category}</th>
                <th>{t.path}</th>
                <th>{t.action}</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredEntries as entry}
                <tr class:missing={!entry.exists}>
                  <td class="sticky-check"><input type="checkbox" checked={selectedIds.has(entry.id)} onchange={() => toggleSelected(entry)} /></td>
                  <td class="sticky-name">
                    <button class="cell-button name-cell" onclick={() => deepScan(entry)}>
                      <span>{label(entry.nameKey)}</span>
                      <span class={`risk-badge ${entry.risk}`}>{t[entry.risk]}</span>
                    </button>
                  </td>
                  <td><span class:loading-size={refreshingSize.has(entry.id)} class="size-state"><strong>{sizeText(entry)}</strong> <span class="muted">{entry.sizeApproximate ? t.approx : t.exact}</span></span></td>
                  <td>{t[entry.category]}</td>
                  <td><code>{entry.path}</code></td>
                  <td class="row-actions">
                    <button class="link-button icon-only" title={t.openFolder} aria-label={t.openFolder} onclick={() => openPath(entry.path)} disabled={!entry.exists}><FolderOpen size={15} /></button>
                    <button class="link-button icon-only" title={t.refreshSize} aria-label={t.refreshSize} onclick={() => refreshEntrySize(entry)} disabled={!entry.exists || refreshingSize.has(entry.id)}><RefreshCw size={15} /></button>
                    <button class="link-button icon-text" onclick={() => deepScan(entry)} disabled={!entry.exists || deepScanning.has(entry.id)}><ScanSearch size={15} />{deepScanning.has(entry.id) ? t.deepScanning : t.deepScan}</button>
                    <button class="danger-button icon-text" onclick={() => quarantineEntry(entry)} disabled={!entry.exists || entry.risk !== 'cleanable' || deleting.has(entry.id)}><ShieldCheck size={15} />{deleting.has(entry.id) ? t.deleting : t.quarantineAction}</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </section>

  </section>
  {:else}
    <section class="deep-page">
      {#if deepEntry}
        <div class="deep-header">
          <div>
            <span class={`risk-badge ${deepEntry.risk}`}>{t[deepEntry.risk]}</span>
            <h2>{label(deepEntry.nameKey)}</h2>
            <code>{deepEntry.path}</code>
          </div>
          <div class="deep-actions">
            <button class="secondary icon-text" onclick={() => openPath(deepEntry!.path)} disabled={!deepEntry.exists}><FolderOpen size={15} />{t.openFolder}</button>
            <button class="secondary icon-text" onclick={() => deepScan(deepEntry!)} disabled={!deepEntry.exists || deepScanning.has(deepEntry.id)}><ScanSearch size={15} />{deepScanning.has(deepEntry.id) ? t.deepScanning : t.deepScan}</button>
            <button class="danger-button icon-text" onclick={() => quarantineEntry(deepEntry!)} disabled={!deepEntry.exists || deepEntry.risk !== 'cleanable' || deleting.has(deepEntry.id)}><ShieldCheck size={15} />{t.quarantineAction}</button>
          </div>
        </div>
        <section class="deep-metrics">
          <div><span>{t.size}</span><strong>{sizeText(deepEntry)}</strong></div>
          <div><span>{t.status}</span><strong>{deepEntry.exists ? t.exists : t.missing}</strong></div>
          <div><span>{t.children}</span><strong>{childEntries.length}</strong></div>
          <div><span>{t.category}</span><strong>{t[deepEntry.category]}</strong></div>
        </section>
        <section class="deep-grid">
          <div class="deep-card">
            <h2>{t.childRanking}</h2>
            {#each childEntries as child}
              <div class="deep-child">
                <div>
                  <strong>{label(child.nameKey)}</strong>
                  <code>{child.path}</code>
                </div>
                <span>{sizeText(child)}</span>
                <button class="link-button icon-only" title={t.openFolder} aria-label={t.openFolder} onclick={() => openPath(child.path)} disabled={!child.exists}><FolderOpen size={14} /></button>
              </div>
            {/each}
          </div>
          <div class="deep-card">
            <h2>{t.quickActions}</h2>
            <button class="secondary icon-text" onclick={() => selectCleanableInView()}><ShieldCheck size={15} />{t.selectCleanable}</button>
            <button class="secondary icon-text" onclick={smartDeepScan}><FolderSearch size={15} />{t.smartDeepScan}</button>
            <button class="secondary" onclick={() => (pageMode = 'main')}>{t.back}</button>
          </div>
        </section>
      {:else}
        <p class="empty">{t.empty}</p>
      {/if}
    </section>
  {/if}

  {#if settingsOpen}
    <div class="modal-backdrop" role="presentation" onclick={() => (settingsOpen = false)}>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1" onclick={stopModalEvent} onkeydown={stopModalEvent}>
        <header class="modal-header">
          <div>
            <h2 id="settings-title">{t.settings}</h2>
            <p>{t.appIntro}</p>
          </div>
          <button class="secondary" onclick={() => (settingsOpen = false)}>{t.close}</button>
        </header>
        <div class="modal-body">
          <nav class="modal-tabs" aria-label={t.settings}>
            <button class:active={settingsTab === 'settings'} onclick={() => (settingsTab = 'settings')}><Languages size={15} />{t.settings}</button>
            <button class:active={settingsTab === 'about'} onclick={() => (settingsTab = 'about')}><Info size={15} />{t.about}</button>
            <button class:active={settingsTab === 'updates'} onclick={() => { settingsTab = 'updates'; if (!updateChecked) void checkForAppUpdate() }}><RefreshCw size={15} />{t.updateCheck}</button>
            <button class:active={settingsTab === 'audit'} onclick={() => (settingsTab = 'audit')}><ClipboardList size={15} />{t.auditLog}</button>
            <button class:active={settingsTab === 'donate'} onclick={() => (settingsTab = 'donate')}><Heart size={15} />{t.donate}</button>
          </nav>
          <div class="modal-content">
            {#if settingsTab === 'settings'}
              <div class="setting-row">
                <div>
                  <h3>{t.languageSetting}</h3>
                  <p>{locale === 'zh-CN' ? '切换后立即生效。' : 'Changes apply immediately.'}</p>
                </div>
                <select bind:value={locale} aria-label={t.languageSetting}>
                  <option value="zh-CN">中文</option>
                  <option value="en-US">English</option>
                </select>
              </div>
              <div class="setting-row">
                <div>
                  <h3>{t.themeStyle}</h3>
                  <p>{themeModeLabel(themeMode)}</p>
                </div>
                <select value={themeMode} onchange={changeThemeMode} aria-label={t.themeStyle}>
                  {#each themeModes as mode}
                    <option value={mode}>{themeModeLabel(mode)}</option>
                  {/each}
                </select>
              </div>
            {:else if settingsTab === 'about'}
              <dl class="about-list">
                  <div><dt>{t.appVersion}</dt><dd>{appMetadata.currentVersion}</dd></div>
                <div><dt>{t.buildType}</dt><dd>{t.portableBuild}</dd></div>
                <div><dt>{t.rules}</dt><dd>{insights?.configRuleCount ?? 0}</dd></div>
                <div><dt>{t.graph}</dt><dd>{insights?.dependencyEdgeCount ?? 0}</dd></div>
              </dl>
            {:else if settingsTab === 'updates'}
              <div class="update-card">
                <div class="update-card-head">
                  <div>
                    <strong>{updateInfo?.updateAvailable ? t.updateAvailable : updateInfo && !updateInfo.error ? t.noUpdateAvailable : t.updateCheck}</strong>
                    <p>{t.updateStatus}</p>
                  </div>
                </div>
                <dl class="update-details">
                  <div><dt>{t.appVersion}</dt><dd>{updateInfo?.currentVersion ?? appMetadata.currentVersion}</dd></div>
                  <div><dt>{t.latestVersion}</dt><dd>{updateInfo?.latestVersion ?? (updateLoading ? t.checkingUpdate : '-')}</dd></div>
                  <div><dt>{t.releaseAsset}</dt><dd>{updateInfo?.assetName ?? '-'}</dd></div>
                  <div><dt>{t.publishedAt}</dt><dd>{updateInfo?.publishedAt ? new Date(updateInfo.publishedAt).toLocaleString(locale) : '-'}</dd></div>
                </dl>
                {#if updateInfo?.error}
                  <p class="message error update-error">{t.updateUnavailable}: {updateInfo.error}</p>
                {/if}
                <div class="update-actions">
                  <button class="secondary icon-text update-action-button" onclick={() => openExternalUrl(appMetadata.officialUrl)}>
                    <Globe size={15} />{t.officialWebsite}
                  </button>
                  <button class="secondary icon-text update-action-button" onclick={() => openExternalUrl(appMetadata.githubUrl)}>
                    <svg class="brand-icon" viewBox="0 0 24 24" aria-hidden="true"><path d={siGithub.path} /></svg>{t.github}
                  </button>
                  <button class="secondary icon-text update-action-button" onclick={() => openExternalUrl(updateInfo?.releaseUrl ?? appMetadata.releasesUrl)}>
                    <ExternalLink size={15} />{t.changelog}
                  </button>
                  {#if updateInfo?.downloadUrl}
                    <button class="secondary icon-text update-action-button" onclick={() => openExternalUrl(updateInfo?.downloadUrl)}>
                      <Download size={15} />{t.openDownloadLink}
                    </button>
                  {/if}
                  <button class="icon-text update-action-button update-primary" onclick={checkForAppUpdate} disabled={updateLoading}>
                    <RefreshCw size={15} />{updateLoading ? t.checkingUpdate : t.checkNow}
                  </button>
                </div>
              </div>
            {:else if settingsTab === 'audit'}
              <div class="audit-list">
                {#if auditLog.length === 0}
                  <p class="empty">{t.noAuditLog}</p>
                {:else}
                  {#each auditLog.slice(0, 80) as item}
                    <div class="audit-row">
                      <div>
                        <strong>{item.action}</strong>
                        <span>{new Date(Number(item.created) * 1000).toLocaleString(locale)}</span>
                      </div>
                      <code>{item.targetPath}</code>
                      <p>{item.detail}</p>
                    </div>
                  {/each}
                {/if}
              </div>
            {:else}
              <div class="donation-panel">
                <img src={donationQr} alt={t.donate} />
                <p>{t.donationNote}</p>
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</main>
