FROM python:3.11-slim

WORKDIR /app

# CPU-only torch is plenty for a 4.4M-param encoder.
ENV PIP_NO_CACHE_DIR=1
COPY requirements.txt .
RUN pip install --index-url https://download.pytorch.org/whl/cpu torch \
 && pip install transformers fastapi "uvicorn[standard]" "pydantic>=2"

COPY model/ /app/model/
COPY app.py /app/app.py

EXPOSE 8077
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8077"]
