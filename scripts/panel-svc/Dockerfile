FROM python:3.11-slim
WORKDIR /app
ENV PIP_NO_CACHE_DIR=1
# Embedding-head router: numpy-only inference (embeddings come from LiteLLM), no torch.
RUN pip install numpy requests fastapi "uvicorn[standard]" "pydantic>=2"
COPY router-head.json /app/router-head.json
COPY panel_features.py /app/panel_features.py
COPY embedding_infer.py /app/embedding_infer.py
COPY app.py /app/app.py
EXPOSE 8078
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8078"]
