// Écran des particules : catalogue fourni par l'application, un interrupteur "je connais" par
// emploi. Seuls les emplois cochés sont autorisés dans les exercices d'IA (D38 A).

import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { ParticleUsage } from "../types";

// Emplois regroupés par particule, dans l'ordre du catalogue (la liste arrive déjà triée).
function groupByParticle(usages: ParticleUsage[]): { particle: string; usages: ParticleUsage[] }[] {
  const groups: { particle: string; usages: ParticleUsage[] }[] = [];
  for (const usage of usages) {
    const last = groups[groups.length - 1];
    if (last && last.particle === usage.particle) {
      last.usages.push(usage);
    } else {
      groups.push({ particle: usage.particle, usages: [usage] });
    }
  }
  return groups;
}

export default function ParticlesPage() {
  const [usages, setUsages] = useState<ParticleUsage[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    try {
      setUsages(await api.listParticleUsages());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  useEffect(() => {
    reload();
  }, []);

  async function toggleUsage(usage: ParticleUsage) {
    try {
      const updated = await api.setParticleUsageActive(usage.id, !usage.active);
      setUsages((current) => current.map((u) => (u.id === updated.id ? updated : u)));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function setParticle(particle: string, active: boolean) {
    try {
      await api.setParticleActive(particle, active);
      await reload();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  const knownCount = usages.filter((u) => u.active).length;

  return (
    <section className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Particules</h2>
        <p className="text-sm text-stone-500">
          Coche les emplois que tu connais : l'IA n'utilisera que ceux-là. Une particule dont
          aucun emploi n'est coché est signalée si elle apparaît dans une phrase. ({knownCount} /{" "}
          {usages.length} emplois connus)
        </p>
      </div>

      {error && <p className="error">{error}</p>}

      {groupByParticle(usages).map((group) => {
        const allActive = group.usages.every((u) => u.active);
        const activeCount = group.usages.filter((u) => u.active).length;
        return (
          <div key={group.particle} className="card space-y-3">
            <div className="flex items-center gap-3">
              <h3 className="text-2xl font-bold">{group.particle}</h3>
              <span className="text-sm text-stone-500">
                {activeCount} / {group.usages.length} emploi(s) connu(s)
              </span>
              <button
                className="btn ml-auto"
                onClick={() => setParticle(group.particle, !allActive)}
              >
                {allActive ? "Tout décocher" : "Tout cocher"}
              </button>
            </div>
            <ul className="space-y-2">
              {group.usages.map((usage) => (
                <li
                  key={usage.id}
                  className={`flex items-start gap-3 border-t border-stone-100 pt-2 ${
                    usage.active ? "" : "opacity-60"
                  }`}
                >
                  <input
                    id={`particle-usage-${usage.id}`}
                    type="checkbox"
                    className="mt-1.5 shrink-0"
                    checked={usage.active}
                    onChange={() => toggleUsage(usage)}
                  />
                  <label htmlFor={`particle-usage-${usage.id}`} className="min-w-0 flex-1">
                    <p className="font-medium">{usage.usage}</p>
                    <p className="text-sm">{usage.explanation}</p>
                    <p className="mt-1 text-sm">
                      {usage.example_jp}
                      <span className="text-stone-500"> - {usage.example_fr}</span>
                    </p>
                  </label>
                </li>
              ))}
            </ul>
          </div>
        );
      })}
    </section>
  );
}
