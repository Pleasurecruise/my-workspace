// Shared types — safe to import from both server and client code
export type { Auth, Session, User } from "./server.js";

export {
	authClient,
	signIn,
	signOut,
	signUp,
	useSession,
	getSession,
	createBetterAuthClient,
} from "./client.js";
