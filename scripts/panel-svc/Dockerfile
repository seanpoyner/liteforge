FROM python:3.11-slim
WORKDIR /app
ENV PIP_NO_CACHE_DIR=1
RUN pip install --index-url https://download.pytorch.org/whl/cpu torch \
 && pip install transformers fastapi "uvicorn[standard]" "pydantic>=2"
COPY task_type/ /app/task_type/
COPY difficulty/ /app/difficulty/
COPY reasoning_depth/ /app/reasoning_depth/
COPY context_demand/ /app/context_demand/
COPY fusion.json /app/fusion.json
COPY panel_features.py /app/panel_features.py
COPY panel_infer.py /app/panel_infer.py
COPY app.py /app/app.py
EXPOSE 8078
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8078"]
