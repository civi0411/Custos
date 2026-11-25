import type {
  Task,
  ResearchSynthesisProposal,
  SaveSynthesisProposalParams,
  HandoffToCodingParams,
  HandoffToCodingResult,
} from '@/types/domain';
import type {
  SourceRecord,
  PassageAnchor,
  ResearchClaim,
  ResearchExperimentRun,
  ArtifactLineageNode,
  ResearchRecipe,
  ResearchExecutionRecord,
  CheckpointResumeAssessment,
  AnnotationRecord,
} from '@/types/research';
import {
  fromDaemon,
  toDaemon,
  annotationFromWire,
  annotationToWire,
} from '../normalizers';
import type { DaemonTransport } from './quant_client';

export class ResearchClient {
  constructor(private transport: DaemonTransport) {}

  async listResearchSources(): Promise<SourceRecord[]> {
    return fromDaemon<SourceRecord[]>(
      await this.transport.request<unknown>('v1.research.sources.list', {})
    );
  }

  async saveResearchSource(source: SourceRecord): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>(
      'v1.research.sources.save',
      toDaemon(source)
    );
  }

  async listResearchAnchors(sourceId: string): Promise<PassageAnchor[]> {
    return fromDaemon<PassageAnchor[]>(
      await this.transport.request<unknown>('v1.research.anchors.list', { source_id: sourceId })
    );
  }

  async saveResearchAnchor(anchor: PassageAnchor): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>(
      'v1.research.anchors.save',
      toDaemon(anchor)
    );
  }

  async listResearchClaims(): Promise<ResearchClaim[]> {
    return fromDaemon<ResearchClaim[]>(
      await this.transport.request<unknown>('v1.research.claims.list', {})
    );
  }

  async saveResearchClaim(claim: ResearchClaim): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>(
      'v1.research.claims.save',
      toDaemon(claim)
    );
  }

  async listResearchRuns(): Promise<ResearchExperimentRun[]> {
    return fromDaemon<ResearchExperimentRun[]>(
      await this.transport.request<unknown>('v1.research.runs.list', {})
    );
  }

  async saveResearchRun(run: ResearchExperimentRun): Promise<{ saved: boolean; run_id: string }> {
    return this.transport.request<{ saved: boolean; run_id: string }>(
      'v1.research.runs.save',
      toDaemon(run)
    );
  }

  async listResearchArtifactLineage(artifactPath: string): Promise<ArtifactLineageNode[]> {
    return fromDaemon<ArtifactLineageNode[]>(
      await this.transport.request<unknown>('v1.research.lineage.list', {
        artifact_path: artifactPath,
      })
    );
  }

  async handoffClaimToCoding(
    claimId: string,
    statement: string
  ): Promise<{ handoff_status: string; task: Task }> {
    return this.transport.request<{ handoff_status: string; task: Task }>(
      'v1.research.handoff_coding',
      {
        claim_id: claimId,
        statement,
      }
    );
  }

  async listResearchRecipes(): Promise<ResearchRecipe[]> {
    return fromDaemon<ResearchRecipe[]>(
      await this.transport.request<unknown>('v1.research.recipes.list', {})
    );
  }

  async saveResearchRecipe(recipe: ResearchRecipe): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>(
      'v1.research.recipes.save',
      toDaemon(recipe)
    );
  }

  async getResearchRecipe(recipeId: string): Promise<ResearchRecipe> {
    return fromDaemon<ResearchRecipe>(
      await this.transport.request<unknown>('v1.research.recipes.get', { recipe_id: recipeId })
    );
  }

  async listResearchExecutions(recipeId: string): Promise<ResearchExecutionRecord[]> {
    return fromDaemon<ResearchExecutionRecord[]>(
      await this.transport.request<unknown>('v1.research.executions.list', {
        recipe_id: recipeId,
      })
    );
  }

  async getResearchExecution(executionId: string): Promise<ResearchExecutionRecord> {
    return fromDaemon<ResearchExecutionRecord>(
      await this.transport.request<unknown>('v1.research.executions.get', {
        execution_id: executionId,
      })
    );
  }

  async assessResearchResume(executionId: string): Promise<CheckpointResumeAssessment> {
    return fromDaemon<CheckpointResumeAssessment>(
      await this.transport.request<unknown>('v1.research.executions.resume.assess', {
        execution_id: executionId,
      })
    );
  }

  async listResearchAnnotations(artifactId: string, version?: number): Promise<AnnotationRecord[]> {
    const records = await this.transport.request<unknown[]>('v1.research.annotations.list', {
      artifact_id: artifactId,
      version,
    });
    return records.map(annotationFromWire);
  }

  async saveResearchAnnotation(annotation: AnnotationRecord): Promise<{ saved: boolean; id: string }> {
    return this.transport.request<{ saved: boolean; id: string }>(
      'v1.research.annotations.save',
      annotationToWire(annotation)
    );
  }

  async listSynthesisProposals(): Promise<ResearchSynthesisProposal[]> {
    return this.transport.request<ResearchSynthesisProposal[]>('v1.synthesis.proposals.list', {});
  }

  async getSynthesisProposal(id: string): Promise<ResearchSynthesisProposal | null> {
    return this.transport.request<ResearchSynthesisProposal | null>('v1.synthesis.proposals.get', {
      id,
    });
  }

  async saveSynthesisProposal(
    params: SaveSynthesisProposalParams
  ): Promise<ResearchSynthesisProposal> {
    return this.transport.request<ResearchSynthesisProposal>(
      'v1.synthesis.proposals.save',
      params
    );
  }

  async executeSynthesisHandoff(params: HandoffToCodingParams): Promise<HandoffToCodingResult> {
    return this.transport.request<HandoffToCodingResult>('v1.synthesis.handoff.execute', params);
  }
}
