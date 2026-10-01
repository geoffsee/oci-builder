# Declared before FROM so `--build-arg GREETING=...` can override the default.
# It must be repeated after FROM before later instructions can read it.
ARG GREETING=hello
FROM scratch
ARG GREETING
ENV GREETING=${GREETING}
LABEL org.example.greeting=${GREETING}
WORKDIR /opt/app
COPY message.txt message.txt
