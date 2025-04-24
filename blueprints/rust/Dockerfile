# Minimal runtime container for compiled applications

FROM alpine:3.19

# Set environment variables
ENV TZ=UTC \
    LANG=C.UTF-8

# Install minimal runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tzdata

# Create a non-root user to run the application
RUN addgroup -S appuser && \
    adduser -S -G appuser -h /app appuser && \
    mkdir -p /app && \
    chown -R appuser:appuser /app

# Set working directory
WORKDIR /app

# Copy pre-built binary from local build
COPY ./target/debug/chatbot /app/chatbot

# Switch to non-root user for better security
USER appuser

# Set the binary as the entrypoint
ENTRYPOINT ["/app/chatbot"]

# Metadata labels
LABEL description="Minimal application container" \
      version="1.0"
