FROM scratch AS docs
COPY base.txt /base.txt

FROM scratch AS final
COPY --from=docs /base.txt /base.txt
COPY extra.txt /extra.txt
