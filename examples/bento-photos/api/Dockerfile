FROM node:22-alpine
RUN apk add --no-cache vips-dev
WORKDIR /app
COPY package.json .
RUN npm install --production
COPY server.js .
EXPOSE 8080
CMD ["node", "server.js"]
