import type { ProjectMember } from '@/api/types';
import { splitMentions } from '@/utils/mentions';

/** A comment's text with `@username` mentions of project members highlighted. */
export default function CommentBody({
  body,
  members,
  className,
}: {
  body: string;
  members: ProjectMember[];
  className?: string;
}) {
  const byUsername = new Map(members.map((m) => [m.username.toLowerCase(), m]));
  return (
    <p className={className}>
      {splitMentions(body).map((part, i) => {
        const member = part.kind === 'mention' ? byUsername.get(part.username) : undefined;
        return member ? (
          <span
            key={i}
            title={member.name}
            className="rounded-sm bg-stitch-accent/15 px-0.5 font-semibold text-stitch-accent"
          >
            {part.text}
          </span>
        ) : (
          part.text
        );
      })}
    </p>
  );
}
