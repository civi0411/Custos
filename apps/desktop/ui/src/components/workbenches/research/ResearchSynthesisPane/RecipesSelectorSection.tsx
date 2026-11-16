import React from 'react';
import { FlaskConical, Search } from 'lucide-react';
import type { ResearchRecipe } from '@/types/research';

export interface RecipesSelectorSectionProps {
  recipes: ResearchRecipe[];
  selectedRecipeIds: string[];
  recipeSearch: string;
  isComposing: boolean;
  activeProposalStatus?: string;
  onRecipeSearchChange: (q: string) => void;
  onToggleRecipe: (id: string) => void;
}

export const RecipesSelectorSection: React.FC<RecipesSelectorSectionProps> = ({
  recipes,
  selectedRecipeIds,
  recipeSearch,
  isComposing,
  activeProposalStatus,
  onRecipeSearchChange,
  onToggleRecipe,
}) => {
  const isReadOnly = !isComposing && activeProposalStatus === 'handoff_completed';

  return (
    <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
          <FlaskConical size={13} />
          <span>Reproduction Recipes ({selectedRecipeIds.length} Selected)</span>
        </div>
        <div className="relative">
          <Search size={11} className="absolute left-2 top-1.5 text-[var(--color-fg-muted)]" />
          <input
            type="text"
            placeholder="Filter recipes..."
            value={recipeSearch}
            onChange={(e) => onRecipeSearchChange(e.target.value)}
            className="bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded pl-6 pr-2 py-0.5 text-[11px] text-[var(--color-editor-fg)] focus:outline-none"
          />
        </div>
      </div>

      <div className="max-h-56 overflow-y-auto space-y-2 pr-1">
        {recipes.length === 0 ? (
          <p className="text-xs text-[var(--color-fg-muted)] py-3 text-center">
            No reproduction recipes saved in research repository.
          </p>
        ) : (
          recipes
            .filter(
              (r) =>
                recipeSearch === '' ||
                r.name.toLowerCase().includes(recipeSearch.toLowerCase()) ||
                r.command.toLowerCase().includes(recipeSearch.toLowerCase())
            )
            .map((recipe) => {
              const isChecked = selectedRecipeIds.includes(recipe.id);

              return (
                <div
                  key={recipe.id}
                  onClick={() => {
                    if (!isReadOnly) {
                      onToggleRecipe(recipe.id);
                    }
                  }}
                  className={`p-3 rounded-lg border transition cursor-pointer flex items-start gap-3 ${
                    isChecked
                      ? 'bg-[var(--color-surface-2)] border-[var(--workbench-accent)]/60'
                      : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)] hover:border-[var(--color-border-default)]'
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={isChecked}
                    onChange={() => onToggleRecipe(recipe.id)}
                    disabled={isReadOnly}
                    className="mt-0.5 rounded border-[var(--color-border-default)] accent-[var(--workbench-accent)] cursor-pointer"
                  />
                  <div className="flex-1 space-y-1">
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-semibold text-xs text-[var(--color-editor-fg)]">
                        {recipe.name}
                      </span>
                      <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">
                        {recipe.inputs?.length || 0} inputs • {recipe.outputs?.length || 0} outputs
                      </span>
                    </div>
                    <div className="p-2 rounded bg-[var(--color-surface-3)] font-mono text-[11px] text-[var(--color-editor-fg)]">
                      <code>{recipe.command}</code>
                    </div>
                  </div>
                </div>
              );
            })
        )}
      </div>
    </div>
  );
};
