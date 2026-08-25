import { describe, expect, it } from "vite-plus/test";
import type { KnowledgeDocument } from "./consumer";
import { latestNewspaperEditions, newspaperEdition } from "./newspaper";

function document(id: string, tags: string[], createdAt = "2026-08-25T00:00:00.000Z") {
  const document: KnowledgeDocument = {
    id,
    slug: id,
    title: id,
    summary: id,
    tags,
    visibility: "private",
    contentHash: id,
    createdAt,
    updatedAt: createdAt,
    source: id,
    html: `<p>${id}</p>`,
    toc: [],
  };
  return document;
}

describe("newspaper classification", () => {
  it("uses the explicit Knowledge edition tags", () => {
    expect(newspaperEdition(document("developer", ["daily", "newspaper", "developer-daily"]))).toBe(
      "developer",
    );
    expect(newspaperEdition(document("personal", ["daily", "newspaper", "personal-daily"]))).toBe(
      "personal",
    );
  });

  it("does not treat prompts or untyped newspaper articles as editions", () => {
    expect(
      newspaperEdition(document("prompt", ["daily-prompt", "personal-daily-prompt"])),
    ).toBeNull();
    expect(newspaperEdition(document("untyped", ["daily", "newspaper"]))).toBeNull();
    expect(
      newspaperEdition(document("ambiguous", ["developer-daily", "personal-daily"])),
    ).toBeNull();
  });

  it("selects the latest issue independently for each edition", () => {
    const editions = latestNewspaperEditions([
      document("older-personal", ["personal-daily"], "2026-08-23T00:00:00.000Z"),
      document("developer", ["developer-daily"], "2026-08-25T00:00:00.000Z"),
      document("personal", ["personal-daily"], "2026-08-24T00:00:00.000Z"),
    ]);
    expect(editions.developer?.id).toBe("developer");
    expect(editions.personal?.id).toBe("personal");
  });
});
