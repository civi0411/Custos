import { useState, useCallback, useEffect } from 'react';
import { daemonClient } from '@/api/daemon_client';
import type {
  ResearchSynthesisProposal,
  ExecutionWorkspace,
} from '@/types/domain';
import type { ResearchClaim, ResearchRecipe } from '@/types/research';

export function useHandoffData(onShowToast?: (msg: string) => void) {
  const [proposals, setProposals] = useState<ResearchSynthesisProposal[]>([]);
  const [claims, setClaims] = useState<ResearchClaim[]>([]);
  const [recipes, setRecipes] = useState<ResearchRecipe[]>([]);
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const [proposalsList, claimsList, recipesList, workspacesList] = await Promise.all([
        daemonClient.listSynthesisProposals(),
        daemonClient.listResearchClaims(),
        daemonClient.listResearchRecipes(),
        daemonClient.listWorkspaces(),
      ]);

      setProposals(proposalsList);
      setClaims(claimsList);
      setRecipes(recipesList);
      setWorkspaces(workspacesList);
      return {
        proposals: proposalsList,
        claims: claimsList,
        recipes: recipesList,
        workspaces: workspacesList,
      };
    } catch (err) {
      console.error('Failed to load synthesis workbench data:', err);
      const msg = err instanceof Error ? err.message : String(err);
      setLoadError(msg);
      onShowToast?.(`Failed to load synthesis workbench data: ${msg}`);
      return null;
    } finally {
      setLoading(false);
    }
  }, [onShowToast]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  return {
    proposals,
    setProposals,
    claims,
    recipes,
    workspaces,
    loading,
    loadError,
    loadData,
  };
}
