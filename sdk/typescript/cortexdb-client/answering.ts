import { buildRetrieveContextAql, buildVerifyFactAql } from "./aql";
import { buildGroundedAnswerResponse } from "./grounding";
import type {
  ContextPackResponse,
  GroundedAnswerOptions,
  GroundedAnswerResponse,
  VerificationReportResponse,
} from "./types";

export interface GroundedAnswerClient {
  retrieveContext(scope: string, statement: string): Promise<ContextPackResponse>;
  verifyFact(scope: string, statement: string): Promise<VerificationReportResponse>;
}

export async function answerWithGroundedContext(
  client: GroundedAnswerClient,
  scope: string,
  brain: string,
  question: string,
  answerer: (context: ContextPackResponse) => string | Promise<string>,
  options: GroundedAnswerOptions = {},
): Promise<GroundedAnswerResponse> {
  const requireCitations = options.requireCitations ?? true;
  const retrieveStatement = buildRetrieveContextAql(question, brain, {
    ...options,
    requireCitations,
  });
  const context = await client.retrieveContext(scope, retrieveStatement);
  const answer = await answerer(context);
  const verifyAnswer = options.verifyAnswer ?? true;
  const verifyStatement = verifyAnswer && answer.trim().length > 0
    ? buildVerifyFactAql(answer, brain)
    : null;
  const verification = verifyStatement ? await client.verifyFact(scope, verifyStatement) : null;
  return buildGroundedAnswerResponse({
    question,
    answer,
    retrieveStatement,
    verifyStatement,
    context,
    verification,
    requireCitations,
    minSpanSupportQ16: options.minSpanSupportQ16,
    rejectUnsupported: options.rejectUnsupported,
  });
}
