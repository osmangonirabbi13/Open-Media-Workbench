/**
 * Types for Bilingual Subtitle Review, Glossary Management,
 * Terminology Protection, and Translation Memory.
 */

export type ReviewStatus =
  | 'Pending'
  | 'NeedsReview'
  | 'Reviewed'
  | 'Approved'
  | 'Rejected';

export type QualitySeverity = 'Info' | 'Warning' | 'Error';

export type QualityFindingCode =
  | 'MissingTranslation'
  | 'EmptyTargetText'
  | 'DuplicateTargetCue'
  | 'SourceTargetTimingMismatch'
  | 'UnmappedCue'
  | 'PossibleUntranslatedText'
  | 'ProtectedTermChanged'
  | 'PreferredTermMissing'
  | 'ExcessiveReadingSpeed'
  | 'ExcessiveLineLength'
  | 'LanguagePairUnverified'
  | 'StaleReviewApproval'
  | 'MissingGlossaryReference';

export interface QualityFinding {
  code: QualityFindingCode;
  severity: QualitySeverity;
  message: string;
  details?: string;
}

export interface TranslationReviewEntry {
  id: string;
  sourceCueId: string;
  targetCueId: string;
  sourceText: string;
  targetText: string;
  sourceStartUs: number;
  sourceEndUs: number;
  targetStartUs: number;
  targetEndUs: number;
  status: ReviewStatus;
  findings: QualityFinding[];
  approvedFingerprint?: string;
  approvedAtEpochMs?: number;
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
}

export interface TranslationReviewDocument {
  schemaVersion: number;
  reviewId: string;
  documentId: string;
  sourceTrackId: string;
  targetTrackId: string;
  sourceLanguage: string;
  targetLanguage: string;
  sourceRevision: string;
  targetRevision: string;
  entries: TranslationReviewEntry[];
  referencedGlossaryIds: string[];
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
}

export interface ReviewProgressStats {
  totalMappedCues: number;
  pendingCount: number;
  needsReviewCount: number;
  reviewedCount: number;
  approvedCount: number;
  rejectedCount: number;
  cuesWithWarningsCount: number;
  cuesWithErrorsCount: number;
  unmappedCuesCount: number;
  reviewedPercentage: number;
  approvedPercentage: number;
}

export interface GlossaryEntry {
  id: string;
  sourceTerm: string;
  preferredTerm: string;
  description?: string;
  isProtected: boolean;
  caseSensitive: boolean;
  domain?: string;
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
}

export interface Glossary {
  schemaVersion: number;
  id: string;
  name: string;
  sourceLanguage: string;
  targetLanguage: string;
  isGlobal: boolean;
  projectAssociationId?: string;
  entries: GlossaryEntry[];
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
}

export interface TranslationMemoryEntry {
  id: string;
  sourceTextNormalized: string;
  targetText: string;
  sourceLanguage: string;
  targetLanguage: string;
  domain?: string;
  usageCount: number;
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
}

export interface TranslationMemory {
  schemaVersion: number;
  entries: TranslationMemoryEntry[];
  updatedAtEpochMs: number;
}
