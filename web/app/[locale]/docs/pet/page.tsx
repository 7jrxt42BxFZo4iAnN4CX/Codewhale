import { DocArticle } from "../_components/doc-article";
import { getDocsPet } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsPet(locale);
  return buildPageMetadata({
    path: "/docs/pet",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function PetPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  return <DocArticle t={getDocsPet(locale)} locale={locale} />;
}
