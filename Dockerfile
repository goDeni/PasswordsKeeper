FROM python:3.11.2-slim as build-stage
RUN pip install poetry==1.4.0

WORKDIR /build
RUN \
    --mount=type=bind,source=./src,target=/build/src \
    --mount=type=bind,source=pyproject.toml,target=/build/pyproject.toml \
    poetry build --format wheel --no-ansi

FROM python:3.11.2-slim as final-stage

RUN \
    --mount=type=bind,from=build-stage,source=/build/dist/,target=/tmp/dist \
    pip install /tmp/dist/passwords_keeper-*.whl

WORKDIR /app
CMD [ "passwords_keeper" ]