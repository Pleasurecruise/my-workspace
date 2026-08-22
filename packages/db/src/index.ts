import { PrismaClient } from "./generated/prisma/client.js";
import { PrismaPg } from "@prisma/adapter-pg";
import { env } from "@my-monorepo/env";

const isProduction = process.env.NODE_ENV === "production";

const adapter = new PrismaPg({
	connectionString: env.DATABASE_URL,
});

const globalForPrisma = global as unknown as {
	prisma: PrismaClient;
};

const prisma = globalForPrisma.prisma || new PrismaClient({ adapter });

if (!isProduction) globalForPrisma.prisma = prisma;

export default prisma;
export { prisma };
