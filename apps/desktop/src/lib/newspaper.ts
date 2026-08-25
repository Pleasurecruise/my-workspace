import type { KnowledgeDocument } from "./consumer";

export type NewspaperEdition = "developer" | "personal";

const developerTags = new Set([
  "developer-daily",
  "programmer-daily",
  "newspaper/developer",
  "newspaper/developer-daily",
  "newspaper/programmer",
  "newspaper/programmer-daily",
  "程序员日报",
]);

const personalTags = new Set([
  "personal-daily",
  "newspaper/personal",
  "newspaper/personal-daily",
  "个人日报",
  "每日日报",
]);

export function newspaperEdition(
  document: Pick<KnowledgeDocument, "tags">,
): NewspaperEdition | null {
  const tags = new Set(document.tags.map((tag) => tag.trim().toLowerCase()));
  const developer = [...tags].some((tag) => developerTags.has(tag));
  const personal = [...tags].some((tag) => personalTags.has(tag));

  if (developer === personal) return null;
  return personal ? "personal" : "developer";
}

export function latestNewspaperEditions(documents: KnowledgeDocument[]) {
  const editions: Record<NewspaperEdition, KnowledgeDocument | null> = {
    developer: null,
    personal: null,
  };
  const ordered = [...documents].sort(
    (left, right) => Date.parse(right.createdAt) - Date.parse(left.createdAt),
  );
  for (const document of ordered) {
    const edition = newspaperEdition(document);
    if (edition !== null && editions[edition] === null) editions[edition] = document;
  }
  return editions;
}
