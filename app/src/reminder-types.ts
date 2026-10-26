export interface ReminderOwner { workspaceId:string;nodeId:string }
export interface ReminderSettings {deviceId:string;settingsRevision:number;libraries:{workspaceId:string;root:string;enabled:boolean}[];subscriptions:{owner:ReminderOwner;reminderId:string;enabled:boolean}[]}
export interface ReminderCandidate {root:string;owner:ReminderOwner;taskOpen:boolean;resolution:{state:"ready"|"absent"|"invalid"|"unavailable";reminderId:string|null;triggerInstant:string|null;normalDeliveryUntil:string|null;definitionDigest:string|null;providers:Record<string,string>;diagnostics:string[]}}
/** Actual OS call observation, independent of later ledger/history confirmation. */
export interface ReminderSubmissionObservation {
  startedAt:string;
  returnedAt:string;
  /** Exact monotonic elapsed nanoseconds, encoded as a canonical unsigned integer. */
  elapsedNanoseconds:string;
  result:{kind:"returnedSuccess"}|{kind:"returnedFailure";hresult:number};
}
export interface ReminderDelivery {executionKey:string;delivery:{state:"submittedToSystem"|"failedBeforeCall"|"unknown"|"inAppOnly";at?:string;diagnostic?:string;summaryKey?:string};submissionObservation:ReminderSubmissionObservation|null;handled:boolean;invalidated:"libraryDisabled"|"subscriptionDisabled"|"taskClosed"|"definitionChanged"|null}
export interface ReminderEligibility {executionKey:string;timing:"upcoming"|"due"|"missed";retained:ReminderDelivery|null}
export interface ReminderInspection {candidate:ReminderCandidate;settings:ReminderSettings;delivery:ReminderDelivery|null;eligibility:ReminderEligibility|null;issue:{code:string;message:string}|null}
export interface ReminderRecord {owner:ReminderOwner;reminderId:string;triggerInstant:string;status:ReminderDelivery}
export interface ReminderRecordsPage {records:ReminderRecord[];next:string|null}
export interface ReminderScheduleCursor {settingsRevision:number;after:string}
export interface ReminderSchedulePage {entries:{candidate:ReminderCandidate;eligibility:ReminderEligibility|null;issue:{code:string;message:string}|null}[];issues:{owner:ReminderOwner;reminderId:string;code:string;message:string}[];complete:boolean;next:ReminderScheduleCursor|null}
/** Current desktop-owner observation, separate from the saved task and delivery records. */
export interface ReminderHostStatus {
  desktopActive: boolean;
  notificationState: "unavailable" | "unknown" | "enabled" | "disabledForApplication" | "disabledForUser" | "disabledByPolicy" | "disabledByManifest";
  scheduling: "inactive" | "running" | "paused" | "restricted" | "stopping";
  scanComplete: boolean;
  scopeIssueCount: number;
}
