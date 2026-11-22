// Coque de l'application : en-tête et navigation entre les écrans.

import { useState } from "react";
import ItemsPage from "./components/ItemsPage";
import LessonsPage from "./components/LessonsPage";
import ReviewPage from "./components/ReviewPage";
import "./App.css";

type Page = "review" | "items" | "lessons";

const PAGES: { id: Page; label: string }[] = [
  { id: "review", label: "Réviser" },
  { id: "items", label: "Éléments" },
  { id: "lessons", label: "Leçons" },
];

export default function App() {
  const [page, setPage] = useState<Page>("review");

  return (
    <div className="min-h-screen">
      <header className="border-b border-stone-200 bg-white">
        <div className="mx-auto flex max-w-4xl items-center gap-6 px-6 py-3">
          <h1 className="text-lg font-bold">Kōgakushin</h1>
          <nav className="flex gap-1">
            {PAGES.map((p) => (
              <button
                key={p.id}
                className={page === p.id ? "btn-primary" : "btn"}
                onClick={() => setPage(p.id)}
              >
                {p.label}
              </button>
            ))}
          </nav>
        </div>
      </header>
      {/* Chaque écran recharge ses données à l'ouverture. */}
      <main className="mx-auto max-w-4xl p-6">
        {page === "review" && <ReviewPage />}
        {page === "items" && <ItemsPage />}
        {page === "lessons" && <LessonsPage />}
      </main>
    </div>
  );
}
